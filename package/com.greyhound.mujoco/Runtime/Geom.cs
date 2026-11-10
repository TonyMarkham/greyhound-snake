using System;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // The serialized mirror of the MJCF <geom> element: every attribute
    // of `element geom` in mjcf-3.15.0.schema, in schema order, names
    // verbatim where C# allows, defaults from the schema. `name` is the MJCF
    // name; when empty the exporter uses the owning asset's name. The schema's `plugin`
    // child element is not mirrored.
    //
    // Vectors are stored in Unity axes; the exporter writes them as
    // (x, z, y). Orientation is stored as quat only, as for Site: MJCF's
    // five orientation forms all compile to a unit quaternion.
    //
    // mass has no schema default: 0 = unset, and density applies.
    // Variable-length lists (double[1..n]) are arrays.
    [Serializable]
    public sealed class Geom
    {
        // MJCF name. Empty: the exporter writes the owning asset's name.
        // [Delayed]: applied on Enter or focus loss, because a change
        // renames the asset (ElementEditor).
        [Delayed]
        public string name;

        // Defaults class; empty = none. `class` is a C# keyword.
        public string @class;

        // Null (the default) writes no <geom>; see GeomType.
        public GeomType type = GeomType.Null;

        public int contype = 1;

        public int conaffinity = 1;

        public int condim = 3;

        public int group;

        public int priority;

        // Shape sizes in meters; how many values matter depends on type.
        public Vector3 size;

        // Material asset name; empty = none.
        public string material;

        // [sliding, torsional, rolling], 1 to 3 values.
        public float[] friction = { 1f, 0.005f, 0.0001f };

        public float mass;

        public float density = 1000f;

        public bool shellinertia;

        public float solmix = 1f;

        public float[] solref = { 0.02f, 1f };

        public float[] solimp = { 0.9f, 0.95f, 0.001f, 0.5f, 2f };

        public float margin;

        public float gap;

        // 1 to 6 values; empty = unset.
        public float[] surfacevel = Array.Empty<float>();

        public float adhesion;

        // MJCF fromto (double[6]), split into its two points in meters,
        // written only when useFromto is set. When set, MuJoCo derives
        // pos, quat and size from it.
        public bool useFromto;

        public Vector3 fromtoFrom;

        public Vector3 fromtoTo;

        public Vector3 pos;

        public Quaternion quat = Quaternion.identity;

        // Height-field asset name; required when type is Hfield.
        public string hfield;

        // Mesh asset name; required when type is Mesh.
        public string mesh;

        public float fitscale = 1f;

        public Color rgba = new Color(0.5f, 0.5f, 0.5f, 1f);

        public FluidShape fluidshape;

        public float[] fluidcoef = { 0.5f, 0.25f, 1.5f, 1f, 1f };

        // Custom parameters; MJCF expects size/nuser_geom entries.
        public float[] user = Array.Empty<float>();
    }
}
