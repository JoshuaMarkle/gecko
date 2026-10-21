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
export const IconProfile = () => <Svg><path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11" /><circle cx="10" cy="4.5" r="1.4" fill="currentColor" /><circle cx="5.5" cy="8" r="1.4" fill="currentColor" /><circle cx="9" cy="11.5" r="1.4" fill="currentColor" /></Svg>;
export const IconSearch = () => <Svg><circle cx="8" cy="8" r="2" /><path d="M8 1.8v2M8 12.2v2M1.8 8h2M12.2 8h2M3.6 3.6 5 5M11 11l1.4 1.4M3.6 12.4 5 11M11 5l1.4-1.4" /></Svg>;
export const IconRuns = () => <Svg><path d="M2.5 3.5h11M2.5 8h11M2.5 12.5h7" /><circle cx="12.5" cy="12.5" r="1" fill="currentColor" stroke="none" /></Svg>;
export const IconStats = () => <Svg><path d="M2.5 13.5h11" /><path d="M4.5 11V7.5M8 11V3.5M11.5 11V6" /></Svg>;
export const IconExplore = () => <Svg><circle cx="4" cy="11" r="1.3" /><circle cx="7.5" cy="5" r="1.3" /><circle cx="12" cy="9" r="1.3" /><circle cx="10" cy="13" r="1" fill="currentColor" stroke="none" /><circle cx="4.5" cy="4" r="1" fill="currentColor" stroke="none" /><circle cx="12.5" cy="3.5" r="1" fill="currentColor" stroke="none" /></Svg>;
export const IconPlay = () => <Svg fill><path d="M5 3.2v9.6c0 .4.4.6.7.4l7.2-4.8a.5.5 0 0 0 0-.8L5.7 2.8c-.3-.2-.7 0-.7.4z" /></Svg>;
export const IconPause = () => <Svg fill><rect x="4" y="3" width="2.6" height="10" rx=".6" /><rect x="9.4" y="3" width="2.6" height="10" rx=".6" /></Svg>;
export const IconHome = () => <Svg><path d="M2.5 7.5 8 3l5.5 4.5" /><path d="M4 6.5v7h8v-7" /><path d="M6.8 13.5v-3.2h2.4v3.2" /></Svg>;
export const IconLanguage = () => <Svg><path d="M2 3.5h6M5 2v1.5M3.5 3.5c.5 2.5 2 4.3 4 5.2M6.5 3.5c-.4 2.2-1.8 4.3-4 5.5" /><path d="M8.5 14l2.8-7 2.8 7M9.5 11.8h3.6" /></Svg>;
export const IconStop = () => <Svg fill><rect x="3.5" y="3.5" width="9" height="9" rx="1.2" /></Svg>;
export const IconSummary = () => <Svg><rect x="2" y="2" width="12" height="12" rx="1" /><path d="M2 6h12M2 10h12M6 2v12M10 2v12" /></Svg>;
