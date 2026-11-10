#if GREYHOUND_STEP
using System;

using Greyhound.Step;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // Converts the STEP importer's default geom for a meshed part
    // (StepGeomProperties, an MjGeom payload) into the MuJoCo <geom>
    // payload, in memory at export time. Nothing is saved: the importer
    // regenerates StepGeomProperties on every reimport, so the converted
    // geom follows the import. Geom assets are only for geoms added by
    // hand.
    //
    // Same frame (the part's Unity frame) and units on both sides. Field
    // by field:
    //   type                 -> type, shifted by one: StepGeomType is in
    //                           schema order from 0 (plane = 0), GeomType
    //                           has Null at 0 (plane = 1)
    //   friction  Vector3    -> float[3]
    //   solref    Vector2    -> float[2]
    //   rgba      Vector4    -> Color
    //   fromto    float[6]   -> useFromto + fromtoFrom/fromtoTo; MjGeom's
    //                           null or all-zero fromto means unset
    //   surfacevel, solimp, fluidcoef arrays are copied
    //   every other field is copied as is.
    // MjGeom has no class, material, hfield, mesh or user, so those keep
    // the Geom defaults; the exporter fills mesh from the part's mesh.
    // MjGeom's alternative orientations (axisangle, euler, xyaxes,
    // zaxis) are not converted: only quat is carried. The importer never
    // sets them, so a default geom loses nothing.
    //
    // Optional dependency: compiles only when com.greyhound.step is
    // installed (GREYHOUND_STEP, see StepInertialConverter).
    public static class StepGeomConverter
    {
        public static Geom ToGeom(StepGeomProperties properties)
        {
            MjGeom mj = properties.mj;
            var geom = new Geom
            {
                type = (GeomType)((int)mj.type + 1),
                contype = mj.contype,
                conaffinity = mj.conaffinity,
                condim = mj.condim,
                group = mj.group,
                priority = mj.priority,
                size = mj.size,
                friction = new[] { mj.friction.x, mj.friction.y, mj.friction.z },
                mass = mj.mass,
                density = mj.density,
                shellinertia = mj.shellinertia,
                solmix = mj.solmix,
                solref = new[] { mj.solref.x, mj.solref.y },
                solimp = Copy(mj.solimp),
                margin = mj.margin,
                gap = mj.gap,
                surfacevel = Copy(mj.surfacevel),
                adhesion = mj.adhesion,
                pos = mj.pos,
                quat = mj.quat,
                fitscale = mj.fitscale,
                rgba = new Color(mj.rgba.x, mj.rgba.y, mj.rgba.z, mj.rgba.w),
                fluidshape = (FluidShape)(int)mj.fluidshape,
                fluidcoef = Copy(mj.fluidcoef),
            };

            // fromto is unset when null or all zeros (MjGeom's rule).
            if (mj.fromto != null && mj.fromto.Length == 6 && Array.Exists(mj.fromto, value => value != 0f))
            {
                geom.useFromto = true;
                geom.fromtoFrom = new Vector3(mj.fromto[0], mj.fromto[1], mj.fromto[2]);
                geom.fromtoTo = new Vector3(mj.fromto[3], mj.fromto[4], mj.fromto[5]);
            }
            return geom;
        }

        private static float[] Copy(float[] values)
        {
            return values != null ? (float[])values.Clone() : Array.Empty<float>();
        }
    }
}
#endif
