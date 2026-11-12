using System.Net.WebSockets;

namespace Greyhound.MuJoCo
{
    // One binary message a StateServer received, handed to its
    // BinaryReceived listeners on the main thread: the bytes as the client
    // sent them, and the way to answer that client. The StateServer does not
    // read the bytes; what they mean is between the client and the listener.
    //
    // Example: a dataset generator's client sends a protobuf message
    // describing a job; the listener parses Payload, does the job, and calls
    // Reply with its result.
    public sealed class BinaryRequest
    {
        private readonly StateConnection m_Connection;

        internal BinaryRequest(StateConnection connection, byte[] payload)
        {
            m_Connection = connection;
            Payload = payload;
        }

        // The message's bytes, the whole message, exactly as received.
        public byte[] Payload { get; }

        // Sends `binary` back to the client that sent this message, as one
        // binary WebSocket message. Returns at once; the send runs after
        // any earlier reply on the same connection. Safe to call later,
        // after the listener has returned (e.g. when a long job finishes).
        public void Reply(byte[] binary)
        {
            m_Connection.Send(binary, WebSocketMessageType.Binary);
        }
    }
}
