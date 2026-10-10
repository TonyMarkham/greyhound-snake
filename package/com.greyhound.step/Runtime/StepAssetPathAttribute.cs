using UnityEngine;

namespace Greyhound.Step
{
    // Marks a string field that stores an asset path; the editor drawer
    // renders it as an asset link to that file instead of a raw string.
    public sealed class StepAssetPathAttribute : PropertyAttribute
    {
    }
}
