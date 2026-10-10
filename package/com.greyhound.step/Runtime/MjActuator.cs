using System;

namespace Greyhound.Step
{
    // The serialized mirror of the MJCF <position> actuator element —
    // attribute names verbatim, nothing Unity-specific. ctrllimited and
    // forcelimited are the spec's own limit switches: explicit false
    // disables clamping even when a range is present, which is what
    // makes the emitted file valid under autolimits without heuristics.
    // ctrlrange is in joint units (meters for a slide target, radians
    // for a hinge); forcerange clamps in N or N·m. Like MjJoint, this
    // class is the future seam for MJCF XML serialization: the M3
    // exporter writes XML elements from here.
    [Serializable]
    public sealed class MjActuator
    {
        public StepActuatorType type;

        public bool ctrllimited;

        public float ctrlLo;

        public float ctrlHi;

        public bool forcelimited;

        public float forceLo;

        public float forceHi;
    }
}
