using System;

namespace Greyhound.Step
{
    // The serialized mirror of the MJCF <position> actuator element —
    // attribute names verbatim, nothing Unity-specific. ctrlrange is in
    // joint units (meters for a slide target, radians for a hinge);
    // forcerange clamps in N or N·m. Like MjJoint, this class is the
    // future seam for MJCF XML serialization: the M3 exporter writes
    // XML elements from here.
    [Serializable]
    public sealed class MjActuator
    {
        public StepActuatorType type;

        public float forceLo;

        public float forceHi;

        public float ctrlLo;

        public float ctrlHi;
    }
}
