using System;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // Poses the part it sits on from one MuJoCo joint value (qpos), the way
    // MuJoCo moves the body the joint is defined in. Runtime only: Attach,
    // called by a StateServer in its Awake for each Joint asset it lists,
    // adds it to the joint's part while the game runs, and every field is
    // filled in from that Joint at that moment. Nothing is authored here and
    // nothing is added to the imported model in the editor; the component
    // is gone when Play mode stops.
    //
    //   slide: the part moves q meters along `axis`.
    //   hinge: the part turns q radians about `axis`, through `pos`.
    //   free, ball: take more than one qpos value; never attached.
    //
    // Units match the exporter's <compiler angle="radian">: hinge q in
    // radians, slide q in meters.
    //
    // Hinge sign: the exporter writes a Unity vector (x, y, z) as MJCF
    // (x, z, y) (MjcfExporter.Swap; Unity axis (0, 0, -1) is MJCF
    // "0 -1 0"). That swap is a mirror (determinant -1), and a mirror
    // reverses rotation sense, so +q about the MJCF axis is -q about the
    // Unity axis: A below is the rotation of -q. A slide's translation is
    // unaffected by the mirror and needs no sign.
    //
    // Frame: `pos` and `axis` are authored in the part's own frame (the
    // MJCF body frame), and the exporter writes each part with its
    // localPosition/localRotation, so the driver works on the local pose.
    // The rest pose (t, R) is the part's local pose when the component is
    // added: at a StateServer's Awake, before anything has moved it, i.e.
    // the imported pose, q = 0. With the part's scale taken as 1 (the
    // importer bakes scale into the mesh vertices) and A the rotation of
    // -q about `axis`:
    //   slide: localPosition = t + R * (axis * q),  localRotation = R
    //   hinge: localPosition = t + R * pos - (R * A) * pos,
    //          localRotation = R * A
    // Worked example (hinge): t = 0, R = identity, axis (0, 0, 1),
    // pos (0.1, 0, 0), q = pi/2, so A turns -90 degrees about Unity z
    // (Quaternion.AngleAxis(-90, z) maps (1, 0, 0) to (0, -1, 0)).
    // localPosition = (0.1, 0, 0) - (0, -0.1, 0) = (0.1, 0.1, 0): the
    // part's origin, 0.1 m left of the pivot, swings to 0.1 m above it.
    //
    // One driven joint per part: a second driver on the same part poses
    // it from the same rest pose and overwrites the first.
    [AddComponentMenu("")]
    public sealed class JointDriver : MonoBehaviour
    {
        // MJCF +q about an axis is Unity -q about the same written axis
        // (see Hinge sign).
        private const float MjcfToUnityHingeSign = -1f;

        // Every warning starts with this, so the Console shows who wrote it.
        private const string LogPrefix = "[JointDriver]";

        // The squared length below which an axis counts as zero: a length
        // under 1e-6 (squared, 1e-12), far below any authored unit axis and
        // above float rounding of a zero vector, so MuJoCo's own zero-axis
        // rejection is matched without refusing a short but real axis.
        private const float ZeroAxisSqrMagnitude = 1e-12f;

        // Shown in the Inspector, filled in by Initialize, never authored.
        [SerializeField] private ScriptableObjects.Joint m_Source;
        [SerializeField] private string m_JointName;
        [SerializeField] private JointType m_Type;
        [SerializeField] private Vector3 m_Axis;
        [SerializeField] private Vector3 m_Pos;
        [SerializeField] private bool m_Limited;
        [SerializeField] private float m_RangeLo;
        [SerializeField] private float m_RangeHi;
        [SerializeField] private Vector3 m_RestPosition;
        [SerializeField] private Quaternion m_RestRotation = Quaternion.identity;
        [SerializeField] private float m_Q;

        // Raised after Initialize, Apply and ResetPose, for inspectors.
        public event Action Changed;

        // The Joint asset this component drives; its MJCF joint name is the
        // key a StateServer routes qpos by.
        public ScriptableObjects.Joint Source => m_Source;
        public string JointName => m_JointName;
        public JointType Type => m_Type;
        public float Q => m_Q;

        // Adds the component to `asset`'s part and fills it in, or returns
        // the one already driving that Joint there. The part is found by
        // walking the joint body's sibling indices down from `instance`, the
        // joint's model in the scene (the caller's scene reference, e.g.
        // StateServer.instance); nothing is searched for. Null, with a
        // warning naming why, when not playing, without a Joint, for a joint
        // one qpos value cannot drive, without an instance, or when the
        // path does not fit the instance.
        public static JointDriver Attach(ScriptableObjects.Joint asset, Transform instance)
        {
            if (!Application.isPlaying)
            {
                return null;
            }
            if (asset == null)
            {
                Debug.LogWarning($"{LogPrefix} no Joint asset to drive.");
                return null;
            }
            if (!IsDrivable(asset.joint, out string reason))
            {
                Debug.LogWarning($"{LogPrefix} '{asset.name}' not attached: {reason}.", asset);
                return null;
            }
            if (instance == null)
            {
                Debug.LogWarning($"{LogPrefix} '{asset.name}' not attached: no model instance given.", asset);
                return null;
            }
            Transform part = asset.body?.ResolveIn(instance);
            if (part == null)
            {
                string path = asset.body?.indices == null ? string.Empty : string.Join(",", asset.body.indices);
                Debug.LogWarning(
                    $"{LogPrefix} '{asset.name}' not attached: path [{path}] does not fit '{instance.name}'.",
                    asset);
                return null;
            }
            foreach (JointDriver existing in part.GetComponents<JointDriver>())
            {
                if (existing.Source == asset)
                {
                    return existing;
                }
            }
            var driver = part.gameObject.AddComponent<JointDriver>();
            driver.Initialize(asset);
            return driver;
        }

        // Copies the joint's values from the asset and captures the rest
        // pose. Called once by Attach right after AddComponent.
        private void Initialize(ScriptableObjects.Joint source)
        {
            Joint joint = source.joint;
            m_Source = source;
            m_JointName = joint.name;
            m_Type = joint.type;
            m_Axis = joint.axis.normalized;
            m_Pos = joint.pos;
            m_Limited = IsLimited(joint);
            m_RangeLo = joint.rangeLo;
            m_RangeHi = joint.rangeHi;
            m_RestPosition = transform.localPosition;
            m_RestRotation = transform.localRotation;
            m_Q = 0f;
            Changed?.Invoke();
        }

        // Poses the part for joint value q (meters for a slide, radians for
        // a hinge).
        public void Apply(float q)
        {
            m_Q = q;
            if (m_Type == JointType.Slide)
            {
                transform.localRotation = m_RestRotation;
                transform.localPosition = m_RestPosition + m_RestRotation * (m_Axis * q);
            }
            else
            {
                Quaternion rotation = m_RestRotation * Quaternion.AngleAxis(MjcfToUnityHingeSign * q * Mathf.Rad2Deg, m_Axis);
                transform.localRotation = rotation;
                transform.localPosition = m_RestPosition + m_RestRotation * m_Pos - rotation * m_Pos;
            }
            Changed?.Invoke();
        }

        // Puts the part back at its rest pose (q = 0).
        public void ResetPose()
        {
            m_Q = 0f;
            transform.localPosition = m_RestPosition;
            transform.localRotation = m_RestRotation;
            Changed?.Invoke();
        }

        // The joint's range when it is limited (IsLimited), else false.
        public bool TryGetRange(out float lo, out float hi)
        {
            lo = m_Limited ? m_RangeLo : 0f;
            hi = m_Limited ? m_RangeHi : 0f;
            return m_Limited;
        }

        // Whether MuJoCo applies the joint's range: limited True, or Auto
        // with a range set (the exporter writes a non-zero range and
        // <compiler autolimits="true">). Same rule as the Joint inspector.
        // Example: Auto with range 0..0 is unlimited; Auto with -1..1 is
        // limited.
        public static bool IsLimited(Joint joint)
        {
            return joint.limited == FalseTrueAuto.True
                || (joint.limited == FalseTrueAuto.Auto && (joint.rangeLo != 0f || joint.rangeHi != 0f));
        }

        // Whether one qpos value can drive the joint; `reason` says why
        // not. Only slides and hinges with a non-zero axis (MuJoCo rejects
        // a zero axis too).
        public static bool IsDrivable(Joint joint, out string reason)
        {
            if (joint.type != JointType.Slide && joint.type != JointType.Hinge)
            {
                reason = $"a {joint.type.ToString().ToLowerInvariant()} joint takes more than one qpos value";
                return false;
            }
            if (joint.axis.sqrMagnitude < ZeroAxisSqrMagnitude)
            {
                reason = "the axis is zero";
                return false;
            }
            reason = null;
            return true;
        }
    }
}
