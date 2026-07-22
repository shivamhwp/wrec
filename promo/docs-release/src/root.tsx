import { Composition } from "remotion";
import { DocsRelease } from "./video";
import "./style.css";

export const Root = () => (
  <Composition
    id="DocsRelease"
    component={DocsRelease}
    durationInFrames={900}
    fps={30}
    width={1920}
    height={1080}
  />
);
