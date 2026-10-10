using System;

using UnityEngine;

namespace Greyhound.Step
{
    // The serialized mirror of the MJCF <geom> element — the full
    // 3.15.0 attribute surface, names verbatim, field defaults equal
    // to the spec defaults so a fresh payload is a fresh spec element.
    // The geom is authored per part as a StepGeom asset in the set
    // folder (the joint-family pattern); a meshed part with no authored
    // geom emits no <geom> element at all — absent means absent —
    // while its mesh asset and inertial still export.
    //
    // Deliberately unmirrored, each for a stated reason — no silent
    // omissions: `name` is the asset's own name; `class` is the
    // defaults-class indirection (no <default> tree is authored);
    // `mesh` is derived from the part's MeshFilter and emitted as the
    // exported mesh asset reference; `material`, `hfield`, and `sdf`
    // reference asset kinds we do not author; `user` only exists when
    // the model declares nuser_geom > 0, which we never do. The
    // orientation alternatives (axisangle, euler, xyaxes, zaxis) ARE
    // mirrored below: the spec allows at most one orientation
    // attribute per geom, so `quat` is emitted only when no
    // alternative is authored.
    //
    // mass is optional in the spec: 0 means unset and the attribute is
    // omitted; density then applies. fromto is the capsule/cylinder
    // alternative to size: unset (all zeros) means omitted. Collision
    // is per-geom through contype/conaffinity — there is no global
    // switch; two geoms collide iff their bitmask cross-products are
    // nonzero.
    [Serializable]
    public sealed class MjGeom
    {
        public StepGeomType type = StepGeomType.Sphere;

        public Vector3 pos;

        public Quaternion quat = Quaternion.identity;

        // Alternative frame-orientation mechanisms, at most one authored
        // per the spec (null = unset; quat then applies). axisangle is
        // real(4) = axis xyz + angle; euler real(3); xyaxes real(6);
        // zaxis real(3).
        public float[] axisangle;

        public float[] euler;

        public float[] xyaxes;

        public float[] zaxis;

        public Vector3 size;

        public int contype = 1;

        public int conaffinity = 1;

        public int condim = 3;

        public int group;

        public int priority;

        public Vector3 friction = new Vector3(1f, 0.005f, 0.0001f);

        public float solmix = 1f;

        public Vector2 solref = new Vector2(0.02f, 1f);

        public float[] solimp = { 0.9f, 0.95f, 0.001f, 0.5f, 2f };

        public float margin;

        public float gap;

        public float mass;

        public float density = 1000f;

        public Vector4 rgba = new Vector4(0.5f, 0.5f, 0.5f, 1f);

        public bool shellinertia;

        public float[] fromto;

        public float fitscale = 1f;

        public StepFluidShape fluidshape;

        public float[] fluidcoef = { 0.5f, 0.25f, 1.5f, 1f, 1f };

        public float[] surfacevel = new float[6];

        public float adhesion;
    }
}
