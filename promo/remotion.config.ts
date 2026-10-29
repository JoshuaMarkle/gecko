import { Config } from "@remotion/cli/config";

// WebGL in headless renders (stills, the final render): ANGLE.
Config.setChromiumOpenGlRenderer("angle");
Config.setVideoImageFormat("jpeg");
