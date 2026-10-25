using UnityEngine;

namespace Greyhound.Step
{
    // One site on a STEP part — the authored MJCF <site> element: a
    // named point that moves with its part, for a sensor to measure.
    // Root points at the owning StepJointSet (the required convenience
    // back-reference, durable because .asset files are not reimported);
    // `body` is the part's identity (StepSiblingPath: the sibling-index
    // path from the assembly root, creation-time name as a display
    // reference). Any part may carry one, the assembly root included:
    // with rootMobility Welded the root is fixed to the world, so a site
    // on it is a fixed point in the world. The MJCF payload lives in
    // `mj` (MjSite); the .name is the MJCF site name.
    [CreateAssetMenu(fileName = "StepSite", menuName = "Greyhound/Step Site")]
    public sealed class StepSite : ScriptableObject
    {
        public StepJointSet root;

        public StepSiblingPath body;

        public MjSite mj = new MjSite();
    }
}
