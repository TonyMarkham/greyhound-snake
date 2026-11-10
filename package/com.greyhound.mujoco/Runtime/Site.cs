using System;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // The serialized mirror of the MJCF <site> element — attribute
    // names verbatim where C# allows, nothing Unity-specific. A site is a
    // named frame fixed in its body's frame: it has no mass and takes no
    // part in collisions, so adding one changes nothing about the
    // physics. `name` is the MJCF name; when empty the exporter uses the owning asset's name.
    // Every default below is MuJoCo's own (mujoco-schema.md, body/site).
    [Serializable]
    public sealed class Site
    {
        // MJCF name. Empty: the exporter writes the owning asset's name.
        // [Delayed]: applied on Enter or focus loss, because a change
        // renames the asset (ElementEditor).
        [Delayed]
        public string name;

        // Defaults class for attributes left unspecified; empty = none.
        // `class` is a C# keyword, hence the @.
        public string @class;

        // Shape for rendering, and the active zone of a touch sensor.
        public SiteType type = SiteType.Sphere;

        // Visualizer show/hide group, or a custom tag.
        public int group;

        // Material asset name; empty = none.
        public string material;

        // Mesh asset name; required when type is Mesh, else ignored.
        public string mesh;

        // Colour; when it differs from this default it overrides the
        // material's colour.
        public Color rgba = new Color(0.5f, 0.5f, 0.5f, 1f);

        // Shape sizes in meters. The docs give no per-type meaning for
        // sites; the geom table (sphere radius, box half-sizes, ...) is
        // the likely reading but is not stated.
        public Vector3 size = new Vector3(0.005f, 0.005f, 0.005f);

        // MJCF fromto (real(6)), split into its two points in meters, and
        // written only when useFromto is set. Valid for capsule,
        // ellipsoid, cylinder and box; +Z runs from fromtoFrom to
        // fromtoTo and the frame sits at their midpoint. When set, MuJoCo
        // ignores pos and quat.
        public bool useFromto;

        public Vector3 fromtoFrom;

        public Vector3 fromtoTo;

        // Position in meters, in the frame of the body where the site is
        // defined.
        public Vector3 pos;

        // Orientation in the body's frame. MJCF accepts five forms
        // (quat, axisangle, xyaxes, zaxis, euler) and stores all of them
        // as a unit quaternion, so only quat is kept. Identity is
        // MuJoCo's default: the site's axes are its body's.
        public Quaternion quat = Quaternion.identity;

        // Custom parameters; MJCF expects size/nuser_site entries.
        public float[] user = Array.Empty<float>();
    }
}
