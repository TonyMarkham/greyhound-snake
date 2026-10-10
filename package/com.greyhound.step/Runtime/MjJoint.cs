using System;

using UnityEngine;

namespace Greyhound.Step
{
    // The serialized mirror of the MJCF <joint> element — attribute
    // names verbatim, nothing Unity-specific. limited is the spec's own
    // limit switch — when true, the range attribute limits the joint
    // and the range fields are the limits. Units are MJCF-native: pos
    // and slide range in meters, hinge range in radians, damping in
    // N·s/m or N·m·s/rad, frictionloss in N or N·m, armature in kg·m²
    // or kg. pos and axis are in the frame of the body where the joint
    // is defined; pos is render-only for slides. The Mj* payload
    // classes are the future seam for MJCF XML serialization: the M3
    // exporter writes XML elements from here.
    [Serializable]
    public sealed class MjJoint
    {
        public StepJointType type;

        public Vector3 pos;

        public Vector3 axis;

        public bool limited;

        public float rangeLo;

        public float rangeHi;

        public float damping;

        public float frictionloss;

        public float armature;
    }
}
