using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Net.WebSockets;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace Greyhound.MuJoCo
{
    // The StateServer's WebSocket endpoint: ws://<address>:<port>/ (any
    // path), on the address the StateServer chose (loopback by default).
    // Every message a client sends, text or binary, is queued whole, as
    // received, as a ReceivedMessage carrying the connection it came on, so
    // the StateServer can reply on it; the StateServer handles the queue on
    // Unity's main thread, because Transforms can only be moved there. Text
    // and binary messages share the one queue, so they are handled in the
    // order they arrived. Any number of clients may connect.
    //
    // Why not HttpListener: Unity's Mono HttpListener answers a valid
    // WebSocket upgrade request with 400 (IsWebSocketRequest is false), so
    // this listens on a plain TcpListener and does the RFC 6455 opening
    // handshake itself, then hands the stream to .NET's WebSocket
    // (WebSocket.CreateFromStream) for framing.
    //
    // Handshake, checkable by hand (RFC 6455 section 1.3): the client's
    // Sec-WebSocket-Key is concatenated with the fixed GUID below, SHA-1
    // hashed, and base64 encoded into Sec-WebSocket-Accept. Example from
    // the RFC: key "dGhlIHNhbXBsZSBub25jZQ==" gives accept
    // "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".
    //
    // Runs on thread-pool threads, not Unity's main thread, so receiving
    // never waits on a frame. Dispose stops the listener and ends every
    // connection.
    internal sealed class StateSocket : IDisposable
    {
        private const string HandshakeGuid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

        // One read; a longer message is assembled from several reads.
        private const int ReceiveBufferBytes = 16 * 1024;

        // The opening request is a few hundred bytes; anything past this is
        // not a WebSocket client.
        private const int MaxHandshakeBytes = 8 * 1024;

        private readonly TcpListener m_Listener;
        private readonly CancellationTokenSource m_Cancel = new CancellationTokenSource();
        private readonly BlockingCollection<ReceivedMessage> m_Messages;
        private int m_Clients;
        private volatile bool m_Listening;
        private volatile string m_LastError;

        public StateSocket(IPAddress address, int port, BlockingCollection<ReceivedMessage> messages)
        {
            Address = address;
            Port = port;
            m_Messages = messages;
            m_Listener = new TcpListener(address, port);
        }

        public IPAddress Address { get; }
        public int Port { get; }
        public int Clients => Volatile.Read(ref m_Clients);
        public bool Listening => m_Listening;
        public string LastError => m_LastError;

        // Starts listening; false (and LastError) when the port cannot be
        // bound, e.g. another process already listens on it.
        public bool Start()
        {
            try
            {
                m_Listener.Start();
            }
            catch (SocketException exception)
            {
                m_LastError = exception.Message;
                return false;
            }
            m_Listening = true;
            Task.Run(AcceptLoop);
            return true;
        }

        public void Dispose()
        {
            m_Cancel.Cancel();
            m_Listening = false;
            m_Listener.Stop();
        }

        private async Task AcceptLoop()
        {
            while (!m_Cancel.IsCancellationRequested)
            {
                TcpClient client;
                try
                {
                    client = await m_Listener.AcceptTcpClientAsync();
                }
                catch (Exception) when (m_Cancel.IsCancellationRequested)
                {
                    return;
                }
                catch (SocketException exception)
                {
                    m_LastError = exception.Message;
                    m_Listening = false;
                    return;
                }
                _ = Task.Run(() => Serve(client));
            }
        }

        private async Task Serve(TcpClient client)
        {
            using (client)
            {
                NetworkStream stream = client.GetStream();
                WebSocket socket;
                try
                {
                    string key = await ReadHandshakeKey(stream);
                    if (key == null)
                    {
                        await WriteAscii(stream, "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        return;
                    }
                    await WriteAscii(stream,
                        "HTTP/1.1 101 Switching Protocols\r\n" +
                        "Upgrade: websocket\r\n" +
                        "Connection: Upgrade\r\n" +
                        $"Sec-WebSocket-Accept: {AcceptKey(key)}\r\n\r\n");
                    socket = WebSocket.CreateFromStream(stream, true, null, TimeSpan.FromSeconds(30));
                }
                catch (Exception exception) when (exception is IOException || exception is SocketException)
                {
                    m_LastError = exception.Message;
                    return;
                }

                Interlocked.Increment(ref m_Clients);
                var connection = new StateConnection(socket, m_Cancel.Token, error => m_LastError = error);
                try
                {
                    await Receive(socket, connection);
                }
                finally
                {
                    Interlocked.Decrement(ref m_Clients);
                    socket.Dispose();
                }
            }
        }

        private async Task Receive(WebSocket socket, StateConnection connection)
        {
            var buffer = new byte[ReceiveBufferBytes];
            using (var message = new MemoryStream())
            {
                try
                {
                    while (socket.State == WebSocketState.Open && !m_Cancel.IsCancellationRequested)
                    {
                        WebSocketReceiveResult result = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), m_Cancel.Token);
                        if (result.MessageType == WebSocketMessageType.Close)
                        {
                            await socket.CloseOutputAsync(WebSocketCloseStatus.NormalClosure, string.Empty, CancellationToken.None);
                            return;
                        }
                        message.Write(buffer, 0, result.Count);
                        if (!result.EndOfMessage)
                        {
                            continue;
                        }
                        if (result.MessageType == WebSocketMessageType.Text)
                        {
                            m_Messages.Add(ReceivedMessage.FromText(connection, Encoding.UTF8.GetString(message.GetBuffer(), 0, (int)message.Length)));
                        }
                        else if (result.MessageType == WebSocketMessageType.Binary)
                        {
                            // ToArray copies: the stream's buffer is reused
                            // for the next message.
                            m_Messages.Add(ReceivedMessage.FromBinary(connection, message.ToArray()));
                        }
                        message.SetLength(0);
                    }
                }
                catch (Exception exception) when (exception is OperationCanceledException || exception is ObjectDisposedException)
                {
                    // Dispose: the server is stopping (its queue may already
                    // be disposed when a last message arrives).
                }
                catch (Exception exception) when (exception is WebSocketException || exception is IOException)
                {
                    m_LastError = exception.Message;
                }
            }
        }

        // Reads the opening request up to its blank line, one byte at a
        // time so no WebSocket frame byte is consumed with it. Returns the
        // Sec-WebSocket-Key of a GET with "Upgrade: websocket", else null.
        private static async Task<string> ReadHandshakeKey(NetworkStream stream)
        {
            var bytes = new List<byte>();
            var one = new byte[1];
            while (bytes.Count < MaxHandshakeBytes)
            {
                int read = await stream.ReadAsync(one, 0, 1);
                if (read == 0)
                {
                    return null;
                }
                bytes.Add(one[0]);
                int n = bytes.Count;
                if (n >= 4 && bytes[n - 4] == '\r' && bytes[n - 3] == '\n' && bytes[n - 2] == '\r' && bytes[n - 1] == '\n')
                {
                    break;
                }
            }

            string[] lines = Encoding.ASCII.GetString(bytes.ToArray()).Split(new[] { "\r\n" }, StringSplitOptions.None);
            if (lines.Length == 0 || !lines[0].StartsWith("GET ", StringComparison.Ordinal))
            {
                return null;
            }
            string key = null;
            bool upgrade = false;
            for (int i = 1; i < lines.Length; i++)
            {
                int colon = lines[i].IndexOf(':');
                if (colon <= 0)
                {
                    continue;
                }
                string name = lines[i].Substring(0, colon).Trim();
                string value = lines[i].Substring(colon + 1).Trim();
                if (name.Equals("Upgrade", StringComparison.OrdinalIgnoreCase))
                {
                    upgrade = value.Equals("websocket", StringComparison.OrdinalIgnoreCase);
                }
                else if (name.Equals("Sec-WebSocket-Key", StringComparison.OrdinalIgnoreCase))
                {
                    key = value;
                }
            }
            return upgrade && !string.IsNullOrEmpty(key) ? key : null;
        }

        private static string AcceptKey(string key)
        {
            using (SHA1 sha1 = SHA1.Create())
            {
                return Convert.ToBase64String(sha1.ComputeHash(Encoding.ASCII.GetBytes(key + HandshakeGuid)));
            }
        }

        private static Task WriteAscii(NetworkStream stream, string text)
        {
            byte[] bytes = Encoding.ASCII.GetBytes(text);
            return stream.WriteAsync(bytes, 0, bytes.Length);
        }
    }
}
