import "./index.css";
import { Composition, Still } from "remotion";
import {
  ZSCLIP_AI_TOTAL_FRAMES,
  ZsclipAiCompetition,
} from "./ZsclipAiCompetition";
import { Zsclip09Cover, Zsclip09Preview } from "./Zsclip09Preview";

export const RemotionRoot = () => {
  return (
    <>
      <Composition
        id="Zsclip09Preview"
        component={Zsclip09Preview}
        durationInFrames={1800}
        fps={30}
        width={1920}
        height={1080}
      />
      <Composition
        id="ZsclipAiCompetition"
        component={ZsclipAiCompetition}
        durationInFrames={ZSCLIP_AI_TOTAL_FRAMES}
        fps={30}
        width={1280}
        height={720}
      />
      <Still
        id="Zsclip09Cover"
        component={Zsclip09Cover}
        width={1920}
        height={1080}
      />
    </>
  );
};
