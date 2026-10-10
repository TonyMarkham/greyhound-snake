using System;

namespace Greyhound.Step
{
    // Identity of a joint's part: the path of sibling indices from the
    // assembly root to the part GameObject (root child n, its child m,
    // ...). The imported hierarchy rebuilds deterministically per STEP
    // file, so the index path is rename-robust where a name key is not,
    // and it distinguishes duplicate-named parts for free. `name`
    // carries the creation-time name as a display reference.
    //
    // Deferred design (out of scope for now): on reimport, a conflict
    // resolution flow compares the resolved part's name against the
    // stored one — a rename is accepted with one click, a structural
    // change surfaces for re-picking — instead of silently rebinding
    // onto whatever now sits at the indexed position.
    [Serializable]
    public sealed class StepSiblingPath
    {
        public int[] indices = new int[0];

        public string name;
    }
}
