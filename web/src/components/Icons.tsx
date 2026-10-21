// 16px line icons for the toolbar (currentColor, no icon library).

import type { ReactNode } from "react";

function Svg({ children, fill }: { children: ReactNode; fill?: boolean }) {
  return (
    <svg viewBox="0 0 16 16" width={16} height={16} aria-hidden="true" fill={fill ? "currentColor" : "none"}
      stroke={fill ? "none" : "currentColor"} strokeWidth={1.4} strokeLinecap="round" strokeLinejoin="round">
      {children}
    </svg>
  );
}

export const IconBoard = () => <Svg><rect x="1.5" y="4" width="13" height="8.5" rx="1.5" /><path d="M4 6.8h.01M6.7 6.8h.01M9.3 6.8h.01M12 6.8h.01M5 10h6" /></Svg>;
export const IconHand = () => <Svg><path d="M5 8.5V4a1 1 0 0 1 2 0v3.5M7 7V3a1 1 0 0 1 2 0v4M9 7V3.8a1 1 0 0 1 2 0V8" /><path d="M11 6.5a1 1 0 0 1 2 0V9a5 5 0 0 1-5 5h-.5A4.5 4.5 0 0 1 3.3 11L2.4 9.3a1 1 0 0 1 1.7-1L5 9.5" /></Svg>;
export const IconCorpus = () => <Svg><path d="M4 1.8h5.5L12.5 5v9.2H4z" /><path d="M9.5 1.8V5h3M6 8h4.5M6 10.5h4.5" /></Svg>;
export const IconProfile = () => <Svg><path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11" /><circle cx="10" cy="4.5" r="1.4" fill="currentColor" /><circle cx="5.5" cy="8" r="1.4" fill="currentColor" /><circle cx="9" cy="11.5" r="1.4" fill="currentColor" /></Svg>;
export const IconSearch = () => <Svg><circle cx="8" cy="8" r="2" /><path d="M8 1.8v2M8 12.2v2M1.8 8h2M12.2 8h2M3.6 3.6 5 5M11 11l1.4 1.4M3.6 12.4 5 11M11 5l1.4-1.4" /></Svg>;
export const IconOptimize = () => <Svg><circle cx="8" cy="8" r="5.5" /><circle cx="8" cy="8" r="2.5" /><circle cx="8" cy="8" r=".6" fill="currentColor" stroke="none" /></Svg>;
export const IconRuns = () => <Svg><path d="M2.5 3.5h11M2.5 8h11M2.5 12.5h7" /><circle cx="12.5" cy="12.5" r="1" fill="currentColor" stroke="none" /></Svg>;
export const IconStats = () => <Svg><path d="M2.5 13.5h11" /><path d="M4.5 11V7.5M8 11V3.5M11.5 11V6" /></Svg>;
export const IconExplore = () => <Svg><circle cx="4" cy="11" r="1.3" /><circle cx="7.5" cy="5" r="1.3" /><circle cx="12" cy="9" r="1.3" /><circle cx="10" cy="13" r="1" fill="currentColor" stroke="none" /><circle cx="4.5" cy="4" r="1" fill="currentColor" stroke="none" /><circle cx="12.5" cy="3.5" r="1" fill="currentColor" stroke="none" /></Svg>;
export const IconPlay = () => <Svg fill><path d="M5 3.2v9.6c0 .4.4.6.7.4l7.2-4.8a.5.5 0 0 0 0-.8L5.7 2.8c-.3-.2-.7 0-.7.4z" /></Svg>;
export const IconPause = () => <Svg fill><rect x="4" y="3" width="2.6" height="10" rx=".6" /><rect x="9.4" y="3" width="2.6" height="10" rx=".6" /></Svg>;
export const IconSkip = () => <Svg fill><path d="M3 3.6v8.8c0 .4.4.6.7.4l6.4-4.4a.5.5 0 0 0 0-.8L3.7 3.2c-.3-.2-.7 0-.7.4z" /><rect x="11" y="3" width="2.2" height="10" rx=".6" /></Svg>;
