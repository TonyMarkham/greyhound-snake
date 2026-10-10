using UnityEngine;

namespace Greyhound.Step
{
    // One actuator driving one joint by direct object reference. Root
    // points at the owning StepJointSet — the family's root SO — as a
    // required convenience back-reference. The MJCF payload lives in
    // `mj` (MjActuator); the schema carries nothing deployment-side —
    // calibration and transmission constants enter when their consumer
    // exists, never as schema fields.
    public sealed class StepActuator : ScriptableObject
    {
        public StepJointSet root;

        public StepJoint target;

        public MjActuator mj = new MjActuator();
    }
}
