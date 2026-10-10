using UnityEngine;

namespace Greyhound.Step
{
    // One geom on a STEP part — the authored MJCF <geom> element, the
    // family's collision and appearance surface. Root points at the
    // owning StepJointSet (the required convenience back-reference,
    // durable because .asset files are not reimported); `body` is the
    // part's identity (StepSiblingPath: the sibling-index path from
    // the assembly root, creation-time name as a display reference).
    // The MJCF payload lives in `mj` (MjGeom); the .name is the MJCF
    // geom name. A meshed part with no StepGeom asset contributes no
    // <geom> element at export — absent means absent — while its mesh
    // asset and inertial still export. The mesh reference itself is
    // derived, not authored.
    public sealed class StepGeom : ScriptableObject
    {
        public StepJointSet root;

        public StepSiblingPath body;

        public MjGeom mj = new MjGeom();
    }
}
