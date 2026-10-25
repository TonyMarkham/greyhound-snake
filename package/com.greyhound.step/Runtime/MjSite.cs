using System;

using UnityEngine;

namespace Greyhound.Step
{
    // The serialized mirror of the MJCF <site> element — attribute
    // names verbatim, nothing Unity-specific. A site is a named point
    // fixed in its body's frame: it has no mass and takes no part in
    // collisions, so adding one changes nothing about the physics. pos
    // is in meters, in the frame of the body where the site is defined.
    // No orientation is authored, so the site's axes are its body's: a
    // framepos measured against this site comes out in those axes.
    [Serializable]
    public sealed class MjSite
    {
        public Vector3 pos;
    }
}
