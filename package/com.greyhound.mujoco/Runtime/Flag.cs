using System;

namespace Greyhound.MuJoCo
{
    // The serialized mirror of the MJCF <option><flag> element: every
    // attribute of `element flag` in mjcf-3.15.0.schema, in schema order,
    // with the schema's defaults. The first block switches features off
    // (default enable); the second switches features on (default
    // disable). `override` is a C# keyword.
    [Serializable]
    public sealed class Flag
    {
        public EnableState constraint = EnableState.Enable;
        public EnableState equality = EnableState.Enable;
        public EnableState frictionloss = EnableState.Enable;
        public EnableState limit = EnableState.Enable;
        public EnableState contact = EnableState.Enable;
        public EnableState spring = EnableState.Enable;
        public EnableState damper = EnableState.Enable;
        public EnableState gravity = EnableState.Enable;
        public EnableState clampctrl = EnableState.Enable;
        public EnableState warmstart = EnableState.Enable;
        public EnableState filterparent = EnableState.Enable;
        public EnableState actuation = EnableState.Enable;
        public EnableState refsafe = EnableState.Enable;
        public EnableState sensor = EnableState.Enable;
        public EnableState midphase = EnableState.Enable;
        public EnableState eulerdamp = EnableState.Enable;
        public EnableState autoreset = EnableState.Enable;
        public EnableState nativeccd = EnableState.Enable;
        public EnableState island = EnableState.Enable;
        public EnableState multiccd = EnableState.Enable;

        public EnableState @override = EnableState.Disable;
        public EnableState energy = EnableState.Disable;
        public EnableState fwdinv = EnableState.Disable;
        public EnableState invdiscrete = EnableState.Disable;
        public EnableState sleep = EnableState.Disable;
        public EnableState diagexact = EnableState.Disable;
        public EnableState ipc = EnableState.Disable;
    }
}
