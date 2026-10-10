using UnityEngine;

namespace Greyhound.Step
{
    // One joint on a STEP part. Root points at the owning StepJointSet —
    // the family's root SO — as a required convenience back-reference,
    // durable because .asset files are not reimported; the exporter
    // walks the elements from the set. `body` is the identity of the
    // body the joint is defined in (StepSiblingPath): the sibling-index
    // path from the assembly root as the source of truth, with the
    // creation-time name as a display reference. The MJCF payload lives
    // in `mj` (MjJoint); the .name is the MJCF joint name. Parenting is
    // the hierarchy's job. Units are MJCF-native: pos and slide range
    // in meters, hinge range in radians, damping in N·s/m or N·m·s/rad,
    // frictionloss in N or N·m, armature in kg·m² or kg. pos and axis
    // are in the frame of the body where the joint is defined; pos is
    // render-only for slides.
    public sealed class StepJoint : ScriptableObject
    {
        public StepJointSet root;

        public StepSiblingPath body;

        public MjJoint mj = new MjJoint();
    }
}
