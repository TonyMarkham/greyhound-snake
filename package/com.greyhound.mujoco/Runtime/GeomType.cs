namespace Greyhound.MuJoCo
{
    // Schema enum geomtype (mjcf-3.15.0.schema), in schema order, after
    // one value of our own: Null. Null is not an MJCF type; a Geom asset
    // of type Null writes no <geom>. With overrideDefault on, it is a
    // pure nullifier: the GameObject's MeshFilter mesh is not exported
    // and nothing is added in its place. Null is the field default.
    //
    // Null being first shifts every schema type by one from the schema's
    // own order (plane = 1, ..., sdf = 9); StepGeomConverter accounts
    // for that when mapping step's StepGeomType.
    public enum GeomType
    {
        Null,
        Plane,
        Hfield,
        Sphere,
        Capsule,
        Ellipsoid,
        Cylinder,
        Box,
        Mesh,
        Sdf,
    }
}
