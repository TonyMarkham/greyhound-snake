using System;
using UnityEngine;

namespace Greyhound.MuJoCo
{
    // Identity of a joint's part: the path of sibling indices from the
    // assembly root to the part GameObject (root child n, its child m,
    // ...). The imported hierarchy rebuilds deterministically per STEP
    // file, so the index path is rename-robust where a name key is not,
    // and it distinguishes duplicate-named parts for free. `targetName`
    // carries the creation-time name as a display reference.
    //
    // States of `indices`:
    //   []      no target set yet.
    //   [-1]    the target is `root` itself (SELF).
    //   [2, 0]  root's child 2, then its child 0. Indices are >= 0.
    // SELF is only ever the whole path, never one step of a longer one.
    //
    // Deferred design (out of scope for now): on reimport, a conflict
    // resolution flow compares the resolved part's name against the
    // stored one — a rename is accepted with one click, a structural
    // change surfaces for re-picking — instead of silently rebinding
    // onto whatever now sits at the indexed position.
    [Serializable]
    public sealed class SiblingPath
    {
        // `indices` = [SELF] targets the root itself.
        public const int SELF = -1;

        public GameObject root;
        public int[] indices = Array.Empty<int>();
        public string targetName;
    }
}
