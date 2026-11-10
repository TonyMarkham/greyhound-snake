namespace Greyhound.MuJoCo
{
    // MJCF <site type>: the schema types it as enum<geomtype>
    // (mjcf-3.15.0.schema), so all nine geom keywords are accepted. The
    // rendered XML reference lists only sphere, capsule, ellipsoid,
    // cylinder, box and mesh for sites; the schema is the source of
    // truth. Sphere is MuJoCo's default. Mesh requires Site.mesh.
    //
    // Members are not in schema order: Plane, Hfield and Sdf were added
    // after the first six, and Unity serializes an enum by its integer
    // value, so reordering would change the type of saved assets.
    public enum SiteType
    {
        Sphere,
        Capsule,
        Ellipsoid,
        Cylinder,
        Box,
        Mesh,
        Plane,
        Hfield,
        Sdf,
    }
}
