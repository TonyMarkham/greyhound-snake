using System.Collections.Generic;

using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.MuJoCo
{
    // The State Server inspector. Layout in UI/StateServer.uxml, styles in
    // UI/StateServer.uss, both assigned as this script's default references
    // (its .meta), as for JointEditor.
    //   - Card: `port` and the address clients connect to: in Play mode the
    //     address in use (StateServer.Url, which a -state-server argument
    //     may have set), otherwise the default and the argument that
    //     overrides it.
    //   - Root: the model's Root, whose joints this server drives, and the
    //     model's instance in the scene, the parts' starting point.
    //   - Camera: m_Camera, the camera a message's cameraTransform moves.
    //   - Status, Play mode only: listening, connected clients, messages
    //     applied, the last socket error, and one row per attached driver:
    //     its MJCF joint name and part, an editable q, and Reset to Rest.
    //     q is meters for a slide, radians for a hinge; a limited joint
    //     (JointDriver.IsLimited) gets a slider over its range, an
    //     unlimited one a plain number field, since it has no range to
    //     span. Editing q poses the part the same way a message would.
    // Refreshes on the server's Changed event and on play mode changes,
    // never on a timer. The driver rows are rebuilt only when the set of
    // drivers changes, so a refresh during a drag only updates values.
    [CustomEditor(typeof(StateServer))]
    internal sealed class StateServerEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;

        private VisualElement m_Root;
        private StateServer m_Followed;

        // The driver rows' q fields by joint name, and the joint names they
        // were built for (rebuilt when that set changes).
        private readonly Dictionary<string, BaseField<float>> m_QFields = new Dictionary<string, BaseField<float>>();
        private string m_RowsKey;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StateServerEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StateServerEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);
            m_Root = root;

            foreach (string name in new[] { "status-listening", "status-clients", "status-messages" })
            {
                root.Q<TextField>(name).isReadOnly = true;
            }

            root.TrackPropertyValue(serializedObject.FindProperty("port"), _ => Refresh());
            root.RegisterCallback<AttachToPanelEvent>(_ => EditorApplication.playModeStateChanged += OnPlayModeChanged);
            root.RegisterCallback<DetachFromPanelEvent>(_ =>
            {
                EditorApplication.playModeStateChanged -= OnPlayModeChanged;
                Follow(null);
            });

            Refresh();
            return root;
        }

        private void OnPlayModeChanged(PlayModeStateChange change)
        {
            Refresh();
        }

        private void Follow(StateServer server)
        {
            if (m_Followed == server)
            {
                return;
            }
            if (m_Followed != null)
            {
                m_Followed.Changed -= Refresh;
            }
            m_Followed = server;
            if (m_Followed != null)
            {
                m_Followed.Changed += Refresh;
            }
        }

        private void Refresh()
        {
            if (m_Root == null || target == null)
            {
                return;
            }
            var server = (StateServer)target;
            bool playing = EditorApplication.isPlaying;
            m_Root.Q<Label>("address").text = playing
                ? (server.Url == null ? "Not listening (see the Console)." : $"Clients connect to {server.Url}")
                : $"Clients connect to ws://127.0.0.1:{server.port}/ unless the player is launched with {StateServer.AddressArgument} <port> or <host>:<port>.";

            Show(m_Root.Q<Foldout>("status-section"), playing);
            Show(m_Root.Q<Label>("status-edit-hint"), !playing);
            Follow(playing ? server : null);
            if (!playing)
            {
                m_RowsKey = null;
                m_QFields.Clear();
                m_Root.Q<VisualElement>("status-drivers").Clear();
                return;
            }

            m_Root.Q<TextField>("status-listening").SetValueWithoutNotify(server.Listening ? "yes" : "no");
            m_Root.Q<TextField>("status-clients").SetValueWithoutNotify(server.Clients.ToString());
            m_Root.Q<TextField>("status-messages").SetValueWithoutNotify(server.MessagesApplied.ToString());
            Label error = m_Root.Q<Label>("status-error");
            error.text = string.IsNullOrEmpty(server.LastError) ? string.Empty : $"Last socket error: {server.LastError}";
            Show(error, !string.IsNullOrEmpty(server.LastError));

            string rowsKey = string.Join("|", server.Drivers.Keys);
            if (rowsKey != m_RowsKey)
            {
                BuildRows(server, m_Root.Q<VisualElement>("status-drivers"));
                m_RowsKey = rowsKey;
            }
            foreach (KeyValuePair<string, JointDriver> pair in server.Drivers)
            {
                if (pair.Value != null && m_QFields.TryGetValue(pair.Key, out BaseField<float> field))
                {
                    field.SetValueWithoutNotify(pair.Value.Q);
                }
            }
            Show(m_Root.Q<Label>("status-no-drivers"), server.Drivers.Count == 0);
        }

        // One row per driver: the part (read-only, labelled with the MJCF
        // joint name), the q field, and Reset to Rest.
        private void BuildRows(StateServer server, VisualElement list)
        {
            list.Clear();
            m_QFields.Clear();
            foreach (KeyValuePair<string, JointDriver> pair in server.Drivers)
            {
                JointDriver driver = pair.Value;
                if (driver == null)
                {
                    continue;
                }
                var row = new VisualElement();
                row.AddToClassList("mujoco-driver-row");

                var part = new ObjectField(pair.Key) { objectType = typeof(Transform), value = driver.transform };
                part.SetEnabled(false);
                row.Add(part);

                string label = $"q ({(driver.Type == JointType.Slide ? "m" : "rad")})";
                BaseField<float> q = driver.TryGetRange(out float lo, out float hi) && hi > lo
                    ? new Slider(label, lo, hi) { showInputField = true }
                    : new FloatField(label);
                q.SetValueWithoutNotify(driver.Q);
                q.RegisterValueChangedCallback(change =>
                {
                    if (driver != null)
                    {
                        driver.Apply(change.newValue);
                    }
                });
                row.Add(q);
                m_QFields[pair.Key] = q;

                row.Add(new Button(() =>
                {
                    if (driver != null)
                    {
                        driver.ResetPose();
                    }
                }) { text = "Reset to Rest" });

                list.Add(row);
            }
        }

        private static void Show(VisualElement element, bool visible)
        {
            if (element != null)
            {
                element.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None;
            }
        }
    }
}
