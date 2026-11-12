using System;
using System.IO;
using System.Net.WebSockets;
using System.Threading;
using System.Threading.Tasks;

namespace Greyhound.MuJoCo
{
    // One client's WebSocket on a StateSocket, as far as replies need it.
    // A WebSocket allows one send at a time, so every send is chained after
    // the previous one: replies leave in the order Send was called, which
    // for a lockstep client is the order of its requests. Send returns at
    // once and may be called from any thread.
    //
    // A send that fails (the client went away, the server is stopping) is
    // reported through onError and dropped; later sends on a closed socket
    // are dropped silently.
    internal sealed class StateConnection
    {
        private readonly WebSocket m_Socket;
        private readonly CancellationToken m_Token;
        private readonly Action<string> m_OnError;
        private readonly object m_Lock = new object();
        private Task m_Sending = Task.CompletedTask;

        public StateConnection(WebSocket socket, CancellationToken token, Action<string> onError)
        {
            m_Socket = socket;
            m_Token = token;
            m_OnError = onError;
        }

        public void Send(byte[] data, WebSocketMessageType type)
        {
            lock (m_Lock)
            {
                m_Sending = m_Sending.ContinueWith(
                    _ => SendNow(data, type),
                    CancellationToken.None, TaskContinuationOptions.None, TaskScheduler.Default).Unwrap();
            }
        }

        private async Task SendNow(byte[] data, WebSocketMessageType type)
        {
            if (m_Socket.State != WebSocketState.Open)
            {
                return;
            }
            try
            {
                await m_Socket.SendAsync(new ArraySegment<byte>(data), type, true, m_Token);
            }
            catch (Exception exception) when (
                exception is WebSocketException || exception is IOException ||
                exception is OperationCanceledException || exception is ObjectDisposedException)
            {
                m_OnError(exception.Message);
            }
        }
    }
}
