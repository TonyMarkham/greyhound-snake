using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.Net;
using System.Net.Sockets;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // Plays MuJoCo joint state, received over a WebSocket, back onto the
    // imported model. Add it to an ordinary GameObject in the scene (not
    // to the model) and assign the model's Root and the model's instance in
    // the scene.
    //
    // Awake: each Joint in root.joints (the list the MJCF exporter writes,
    // so the names match the model MuJoCo runs) gets a runtime JointDriver
    // component on its part (JointDriver.Attach), keyed here by MJCF joint
    // name. Each part is found by walking the joint's sibling indices down
    // from `instance`; nothing is searched for by name. Nothing is
    // attached in the editor; the components are gone when Play mode stops.
    // OnEnable / OnDisable: start / stop the WebSocket (StateSocket) on
    // ws://<address>:<port>/.
    // Update: applies every message received since the last frame, in
    // order, on the main thread, and raises StateApplied once per message
    // after its joints are posed, so a listener (e.g. a camera) can render
    // and Reply to the client that sent it.
    //
    // Idle wait: in a player, Update blocks up to
    // playerIdleWaitMilliseconds for the next message instead of returning
    // at once, so a player with nothing to do sleeps instead of spinning
    // frames; a message ends the wait as soon as it arrives. The editor
    // never waits: blocking its main thread would stall the editor itself.
    //
    // Address: `port` (Inspector) on 127.0.0.1 by default, so editor Play
    // mode works with nothing else set. A command-line argument overrides
    // it, so several players launched from one build each listen on their
    // own address:
    //   -state-server <port>          e.g. -state-server 9001
    //   -state-server <host>:<port>   e.g. -state-server 127.0.0.1:9003
    // <host> is an IPv4 address or "localhost" (= 127.0.0.1). A malformed
    // value is an error and the server does not listen, rather than
    // silently falling back to the default port another instance may use.
    //
    // Binary messages: the StateServer does not read them. Each is raised,
    // in arrival order with the state messages, as BinaryReceived on the
    // main thread (a BinaryRequest: the bytes and a Reply); what the bytes
    // mean is the listener's business. With no listener a binary message is
    // dropped, with one warning per StateServer.
    //
    // Message (one WebSocket text message, JSON): an optional id, echoed to
    // listeners as StateRequest.Id, and the joints' values, MJCF qpos per
    // joint name; meters for a slide, radians for a hinge.
    //   {"id": 7, "joints": [{"name": "02-Cart.Slider.joint", "qpos": 0.05},
    //                         {"name": "04-Pole.Hinge.joint",  "qpos": 0.3}]}
    // A joint left out keeps its last pose. A name no driver has is warned
    // about once per name and skipped; a message that is not valid JSON
    // is warned about and skipped, with no StateApplied (so no reply).
    //
    // Camera: a message may also carry the pose of m_Camera (Inspector),
    // applied before StateApplied so the reply is
    // rendered from it. World position in meters and Unity Euler angles in
    // degrees (applied Z, then X, then Y, as Transform.eulerAngles):
    //   {"cameraTransform": {"position": {"x": 0, "y": 1.1, "z": -2.5},
    //                        "eulerRotation": {"x": 0, "y": 0, "z": 0}}}
    // It replaces the camera's pose, and the camera keeps it until another
    // message moves it. A message without it leaves the camera where it is;
    // one with any of the six numbers missing is warned about and the
    // camera is not moved.
    [AddComponentMenu("Greyhound/MuJoCo State Server")]
    public sealed class StateServer : MonoBehaviour
    {
        // The command-line argument that overrides `port` (see Address).
        public const string AddressArgument = "-state-server";

        // The default port, on 127.0.0.1, when no -state-server is given.
        public int port = 5555;

        // The MuJoCo annotation of the driven model; its `joints` are driven.
        public ScriptableObjects.Root root;

        // The model's instance in the scene: the copy of root.root that is
        // driven. A scene reference, so a built player knows it without a
        // search; every part (joint bodies, and anything else addressed by
        // a SiblingPath of this model) is reached by its sibling indices from
        // here. Example: a STEP GameObject that is a child of this one, in a
        // self-contained prefab.
        public Transform instance;

        // How long a player's Update waits for a message (see Idle wait).
        public int playerIdleWaitMilliseconds = 100;

        // The camera a message's cameraTransform moves (see Camera).
        [SerializeField] private Camera m_Camera;

        private readonly Dictionary<string, JointDriver> m_Drivers = new Dictionary<string, JointDriver>();
        private readonly BlockingCollection<ReceivedMessage> m_Messages = new BlockingCollection<ReceivedMessage>();
        private readonly HashSet<string> m_UnknownNames = new HashSet<string>();
        private StateSocket m_Socket;
        private int m_LastClients;
        private bool m_WarnedNoBinaryListener;

        // Raised on the main thread when drivers attach, messages are
        // applied, or the client count changes; for the inspector.
        public event Action Changed;

        // Raised on the main thread once per applied message, after its
        // joints are posed; Reply on the request answers its sender.
        public event Action<StateRequest> StateApplied;

        // Raised on the main thread once per binary message, in arrival
        // order with the state messages; Reply on the request answers its
        // sender (see Binary messages).
        public event Action<BinaryRequest> BinaryReceived;

        // The attached runtime components, by MJCF joint name.
        public IReadOnlyDictionary<string, JointDriver> Drivers => m_Drivers;

        // The address in use while enabled, e.g. "ws://127.0.0.1:9001/";
        // null before OnEnable and when the -state-server value is invalid.
        public string Url { get; private set; }

        public bool Listening => m_Socket != null && m_Socket.Listening;
        public int Clients => m_Socket == null ? 0 : m_Socket.Clients;
        public string LastError => m_Socket?.LastError;
        public int MessagesApplied { get; private set; }

        private void Awake()
        {
            if (root == null)
            {
                Debug.LogError("[StateServer] no Root assigned; no joint is driven.", this);
                Changed?.Invoke();
                return;
            }
            if (instance == null)
            {
                Debug.LogError("[StateServer] no instance assigned (the model in the scene); no joint is driven.", this);
                Changed?.Invoke();
                return;
            }
            foreach (ScriptableObjects.Joint asset in root.joints)
            {
                if (asset == null)
                {
                    continue;
                }
                JointDriver driver = JointDriver.Attach(asset, instance);
                if (driver == null)
                {
                    continue;
                }
                if (m_Drivers.TryGetValue(driver.JointName, out JointDriver other) && other != driver)
                {
                    Debug.LogWarning(
                        $"[StateServer] two drivers share the joint name '{driver.JointName}'; '{asset.name}' is ignored.", this);
                    continue;
                }
                m_Drivers[driver.JointName] = driver;
            }
            Changed?.Invoke();
        }

        private void OnEnable()
        {
            if (!TryResolveAddress(out IPAddress address, out int resolvedPort, out string error))
            {
                Url = null;
                Debug.LogError($"[StateServer] not listening: {error}", this);
                Changed?.Invoke();
                return;
            }
            Url = $"ws://{address}:{resolvedPort}/";
            m_Socket = new StateSocket(address, resolvedPort, m_Messages);
            if (m_Socket.Start())
            {
                Debug.Log($"[StateServer] listening on {Url}", this);
            }
            else
            {
                Debug.LogError($"[StateServer] cannot listen on {Url}: {m_Socket.LastError}", this);
            }
            Changed?.Invoke();
        }

        // The -state-server argument when the command line has one, else
        // `port` on 127.0.0.1.
        private bool TryResolveAddress(out IPAddress address, out int resolvedPort, out string error)
        {
            string[] args = Environment.GetCommandLineArgs();
            for (int i = 0; i < args.Length; i++)
            {
                if (args[i] != AddressArgument)
                {
                    continue;
                }
                if (i + 1 >= args.Length)
                {
                    address = null;
                    resolvedPort = 0;
                    error = $"{AddressArgument} has no value; expected <port> or <host>:<port>";
                    return false;
                }
                return TryParseAddress(args[i + 1], out address, out resolvedPort, out error);
            }
            address = IPAddress.Loopback;
            resolvedPort = port;
            error = null;
            return true;
        }

        // Parses "<port>" (host 127.0.0.1) or "<host>:<port>", <host> an
        // IPv4 address or "localhost", <port> 1..65535.
        // Examples: "9001" -> 127.0.0.1:9001; "localhost:9003" ->
        // 127.0.0.1:9003; "9001x" and "127.0.0.1:0" are rejected.
        public static bool TryParseAddress(string text, out IPAddress address, out int parsedPort, out string error)
        {
            address = IPAddress.Loopback;
            parsedPort = 0;
            string portText = text;
            int colon = text.LastIndexOf(':');
            if (colon >= 0)
            {
                string host = text.Substring(0, colon);
                portText = text.Substring(colon + 1);
                if (host.Equals("localhost", StringComparison.OrdinalIgnoreCase))
                {
                    address = IPAddress.Loopback;
                }
                else if (!IPAddress.TryParse(host, out address) || address.AddressFamily != AddressFamily.InterNetwork)
                {
                    error = $"{AddressArgument} '{text}': '{host}' is not an IPv4 address or localhost";
                    return false;
                }
            }
            if (!int.TryParse(portText, NumberStyles.None, CultureInfo.InvariantCulture, out parsedPort)
                || parsedPort < 1 || parsedPort > 65535)
            {
                error = $"{AddressArgument} '{text}': '{portText}' is not a port (1-65535)";
                return false;
            }
            error = null;
            return true;
        }

        private void OnDisable()
        {
            m_Socket?.Dispose();
            m_Socket = null;
            Url = null;
            Changed?.Invoke();
        }

        private void OnDestroy()
        {
            m_Messages.Dispose();
        }

        private void Update()
        {
            bool changed = false;
            int wait = Application.isEditor ? 0 : Mathf.Max(0, playerIdleWaitMilliseconds);
            if (m_Messages.TryTake(out ReceivedMessage message, wait))
            {
                HandleMessage(message);
                changed = true;
                while (m_Messages.TryTake(out message))
                {
                    HandleMessage(message);
                }
            }
            int clients = Clients;
            if (clients != m_LastClients)
            {
                m_LastClients = clients;
                changed = true;
            }
            if (changed)
            {
                Changed?.Invoke();
            }
        }

        // Applies one joint value by MJCF joint name (main thread only).
        public void Apply(string jointName, float qpos)
        {
            if (jointName != null && m_Drivers.TryGetValue(jointName, out JointDriver driver) && driver != null)
            {
                driver.Apply(qpos);
            }
            else if (m_UnknownNames.Add(jointName ?? string.Empty))
            {
                Debug.LogWarning($"[StateServer] no driver for joint '{jointName}'; its values are skipped.", this);
            }
        }

        private void HandleMessage(ReceivedMessage message)
        {
            if (message.Binary != null)
            {
                RaiseBinary(new BinaryRequest(message.Connection, message.Binary));
            }
            else
            {
                ApplyMessage(new StateRequest(message.Connection, message.Text));
            }
        }

        private void RaiseBinary(BinaryRequest request)
        {
            if (BinaryReceived == null)
            {
                if (!m_WarnedNoBinaryListener)
                {
                    m_WarnedNoBinaryListener = true;
                    Debug.LogWarning("[StateServer] a binary message arrived and nothing listens for one (BinaryReceived); it is dropped, and so are later ones.", this);
                }
                return;
            }
            BinaryReceived.Invoke(request);
        }

        // A message with no "joints" still counts: nothing moves, and the
        // listeners still get StateApplied (a lockstep client still gets its
        // one reply).
        private void ApplyMessage(StateRequest request)
        {
            JointStateMessage message;
            try
            {
                message = JsonUtility.FromJson<JointStateMessage>(request.Json);
            }
            catch (ArgumentException exception)
            {
                Debug.LogWarning($"[StateServer] skipped a message that is not valid JSON: {exception.Message}", this);
                return;
            }
            if (message == null)
            {
                return;
            }
            if (message.joints != null)
            {
                foreach (JointState joint in message.joints)
                {
                    Apply(joint.name, joint.qpos);
                }
            }
            ApplyCamera(message.cameraTransform);
            MessagesApplied++;
            request.Id = message.id;
            StateApplied?.Invoke(request);
        }

        // Moves m_Camera to the message's pose (see Camera). JsonUtility
        // leaves a number the JSON does not give at its initial value, NaN
        // here, so a NaN means "not given".
        private void ApplyCamera(CameraTransform pose)
        {
            if (pose == null || pose.IsEmpty)
            {
                return;
            }
            if (!pose.IsComplete)
            {
                Debug.LogWarning("[StateServer] cameraTransform needs position x, y, z and eulerRotation x, y, z; the camera is not moved.", this);
                return;
            }
            if (m_Camera == null)
            {
                Debug.LogWarning("[StateServer] cameraTransform given but no Camera is assigned; the camera is not moved.", this);
                return;
            }
            m_Camera.transform.SetPositionAndRotation(pose.position, Quaternion.Euler(pose.eulerRotation));
        }

        [Serializable]
        private sealed class JointStateMessage
        {
            public int id;
            public JointState[] joints = new JointState[0];
            public CameraTransform cameraTransform = new CameraTransform();
        }

        [Serializable]
        private sealed class CameraTransform
        {
            public Vector3 position = new Vector3(float.NaN, float.NaN, float.NaN);
            public Vector3 eulerRotation = new Vector3(float.NaN, float.NaN, float.NaN);

            public bool IsEmpty => IsMissing(position) && IsMissing(eulerRotation);

            public bool IsComplete => IsGiven(position) && IsGiven(eulerRotation);

            private static bool IsMissing(Vector3 value)
            {
                return float.IsNaN(value.x) && float.IsNaN(value.y) && float.IsNaN(value.z);
            }

            private static bool IsGiven(Vector3 value)
            {
                return !float.IsNaN(value.x) && !float.IsNaN(value.y) && !float.IsNaN(value.z);
            }
        }

        [Serializable]
        private sealed class JointState
        {
            public string name;
            public float qpos;
        }
    }
}
