import { Composition, Folder } from "remotion";
import { Banner, BANNER } from "./Banner";
import { loadFonts } from "./lib/font";
import { FPS, H, W } from "./lib/tokens";
import { BoardOptPreview } from "./previews/BoardOptPreview";
import { CornePreview } from "./previews/CornePreview";
import { Gallery, GALLERY_FRAMES } from "./previews/Gallery";
import { HANDS_FRAMES, HandsPreview } from "./previews/HandsPreview";
import { LayoutRingPreview } from "./previews/LayoutRingPreview";
import { StlPreview } from "./previews/StlPreview";
import { Promo } from "./Promo";
import { PROMO_FRAMES } from "./storyboard";

loadFonts();

export function Root() {
  return (
    <>
    <Composition id="promo" component={Promo} durationInFrames={PROMO_FRAMES} fps={FPS} width={W} height={H} />
    <Composition id="banner" component={Banner} durationInFrames={1} fps={FPS} width={BANNER.w} height={BANNER.h} />
    <Folder name="components">
      <Composition id="board-opt" component={BoardOptPreview} durationInFrames={FPS * 6} fps={FPS} width={W} height={H} />
      <Composition id="layout-ring" component={LayoutRingPreview} durationInFrames={FPS * 6} fps={FPS} width={W} height={H} />
      <Composition id="gallery" component={Gallery} durationInFrames={GALLERY_FRAMES} fps={FPS} width={W} height={H} />
      <Composition id="stl" component={StlPreview} durationInFrames={FPS * 3} fps={FPS} width={W} height={H} />
      <Composition id="hands" component={HandsPreview} durationInFrames={HANDS_FRAMES} fps={FPS} width={W} height={H} />
      <Composition id="corne" component={CornePreview} durationInFrames={FPS * 10} fps={FPS} width={W} height={H} />
    </Folder>
    </>
  );
}
