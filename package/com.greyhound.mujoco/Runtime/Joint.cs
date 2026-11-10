using System;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // The serialized mirror of the MJCF <joint> element: every attribute
    // of `element joint` in mjcf-3.15.0.schema, in schema order, names
    // verbatim where C# allows. `name` is the MJCF name; when
    // empty the exporter uses the owning asset's name. Defaults are the schema's.
    //
    // Units are MJCF-native: pos and slide range in meters, hinge range
    // in radians, damping in N·s/m or N·m·s/rad, frictionloss in N or
    // N·m, armature in kg·m² or kg.
    //
    // Vectors are stored in Unity axes; the exporter writes them as
    // (x, z, y). So the schema's axis default "0 0 1" is stored as
    // (0, 1, 0).
    //
    // Fixed two-value ranges are split into Lo/Hi fields; attributes the
    // schema types as a variable-length list (double[1..n]) are arrays.
    [Serializable]
    public sealed class Joint
    {
        // MJCF name. Empty: the exporter writes the owning asset's name.
        // [Delayed]: applied on Enter or focus loss, because a change
        // renames the asset (ElementEditor).
        [Delayed]
        public string name;

        // Defaults class; empty = none. `class` is a C# keyword.
        public string @class;

        public JointType type = JointType.Hinge;

        public int group;

        // In the frame of the body the joint is defined in. Render-only
        // for slides.
        public Vector3 pos;

        // Unity axes: (0, 1, 0) here is MJCF "0 0 1". A zero axis is a
        // compile error.
        public Vector3 axis = new Vector3(0f, 1f, 0f);

        // [timeconst, dampratio]. When both are positive MuJoCo computes
        // stiffness and damping from them instead; (0, 0) = unset.
        public Vector2 springdamper;

        public FalseTrueAuto limited = FalseTrueAuto.Auto;

        public FalseTrueAuto actuatorfrclimited = FalseTrueAuto.Auto;

        public float[] solreflimit = { 0.02f, 1f };

        public float[] solimplimit = { 0.9f, 0.95f, 0.001f, 0.5f, 2f };

        public float[] solreffriction = { 0.02f, 1f };

        public float[] solimpfriction = { 0.9f, 0.95f, 0.001f, 0.5f, 2f };

        // Spring polynomial, 1 to 3 coefficients.
        public float[] stiffness = { 0f };

        // range (double[2]).
        public float rangeLo;

        public float rangeHi;

        // actuatorfrcrange (double[2]).
        public float actuatorfrcrangeLo;

        public float actuatorfrcrangeHi;

        public bool actuatorgravcomp;

        public float margin;

        // Joint value of the authored configuration. `ref` is a C#
        // keyword.
        public float @ref;

        public float springref;

        public float armature;

        // Damper polynomial, 1 to 3 coefficients.
        public float[] damping = { 0f };

        public float frictionloss;

        // Custom parameters; MJCF expects size/nuser_jnt entries.
        public float[] user = Array.Empty<float>();
    }
}
