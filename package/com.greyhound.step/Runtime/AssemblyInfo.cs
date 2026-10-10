using System.Runtime.CompilerServices;

// The editor assembly consumes Runtime internals deliberately: the
// exporter reuses InertiaTensorMath's quaternion helper — the same
// matrix-to-quaternion math the mass-properties path and the M1 Rust
// reference implementation use — instead of growing a second copy.
[assembly: InternalsVisibleTo("com.greyhound.step.editor")]
