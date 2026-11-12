using System.Net.WebSockets;

namespace Greyhound.MuJoCo
{
    // One state message a StateServer received, handed to its StateApplied
    // listeners after the message's joints are applied: the message's `id`
    // and the way to answer the client that sent it.
    //
    // Example: a camera listener renders the posed scene and calls
    // Reply(jpeg); the client gets that JPEG as one binary WebSocket
    // message, after every earlier reply on its connection.
    public sealed class StateRequest
    {
        private readonly StateConnection m_Connection;

        internal StateRequest(StateConnection connection, string json)
        {
            m_Connection = connection;
            Json = json;
        }

        // The message's "id" (0 when it has none), as the client sent it.
        public int Id { get; internal set; }

        internal string Json { get; }

        // Sends `binary` back to the client that sent this message, as one
        // binary WebSocket message. Returns at once; the send runs after
        // any earlier reply on the same connection.
        public void Reply(byte[] binary)
        {
            m_Connection.Send(binary, WebSocketMessageType.Binary);
        }
    }
}
