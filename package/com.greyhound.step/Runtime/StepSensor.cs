using UnityEngine;

namespace Greyhound.Step
{
    // One MJCF <framepos> sensor: every step MuJoCo writes the target
    // site's 3D position into mjData.sensordata, in meters — in world
    // coordinates, or, with a reference site, measured in that site's
    // frame (the target's position minus the reference's, in the
    // reference's axes). Worked example: target on a pole tip, reference
    // at the tip's upright place; the output is (0, 0, 0) exactly when
    // the tip is there, and its length is the tip's distance from it.
    // Root points at the owning StepJointSet (the required convenience
    // back-reference); target and reference are direct object
    // references to sites in the same set. The .name is the MJCF sensor
    // name. framepos is the only sensor type the family writes.
    [CreateAssetMenu(fileName = "StepSensor", menuName = "Greyhound/Step Sensor")]
    public sealed class StepSensor : ScriptableObject
    {
        public StepJointSet root;

        public StepSite target;

        // Optional: none measures in world coordinates.
        public StepSite reference;
    }
}
