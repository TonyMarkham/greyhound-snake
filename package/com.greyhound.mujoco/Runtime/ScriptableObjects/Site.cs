using UnityEngine;

namespace Greyhound.MuJoCo.ScriptableObjects
{
    // One site on a part, the MJCF <site>: a named point that moves with
    // its part. The counterpart of StepSite. It belongs to the Root whose
    // `sites` list holds it. `body` is the part the site sits on. The
    // MJCF payload lives in `site` (Greyhound.MuJoCo.Site).
    //
    // Inside this namespace a bare `Site` is this asset class, so the
    // payload type is written as MuJoCo.Site.
    [CreateAssetMenu(fileName = MenuManager.SITE_FILE_NAME, menuName = MenuManager.SITE, order = MenuManager.SITE_ORDER)]
    [Icon("Packages/com.greyhound.mujoco/Editor/Icons/Site.png")]
    public sealed class Site : ScriptableObject
    {
        public SiblingPath body;
        public MuJoCo.Site site = new MuJoCo.Site();
    }
}
