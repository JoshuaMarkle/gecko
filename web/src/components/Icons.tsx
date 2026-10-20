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

export const IconHome = () => <Svg><path d="M2.5 7.2 8 2.5l5.5 4.7" /><path d="M4 6.2V13h3v-3.2h2V13h3V6.2" /></Svg>;
export const IconResults = () => <Svg><path d="M2.5 3.5h11M2.5 8h11M2.5 12.5h7" /><circle cx="12.5" cy="12.5" r="1" fill="currentColor" stroke="none" /></Svg>;
export const IconAnalysis = () => <Svg><path d="M2.5 13.5h11" /><path d="M4.5 11V7.5M8 11V3.5M11.5 11V6" /></Svg>;
export const IconPlay = () => <Svg fill><path d="M5 3.2v9.6c0 .4.4.6.7.4l7.2-4.8a.5.5 0 0 0 0-.8L5.7 2.8c-.3-.2-.7 0-.7.4z" /></Svg>;
export const IconPause = () => <Svg fill><rect x="4" y="3" width="2.6" height="10" rx=".6" /><rect x="9.4" y="3" width="2.6" height="10" rx=".6" /></Svg>;
export const IconSkip = () => <Svg fill><path d="M3 3.6v8.8c0 .4.4.6.7.4l6.4-4.4a.5.5 0 0 0 0-.8L3.7 3.2c-.3-.2-.7 0-.7.4z" /><rect x="11" y="3" width="2.2" height="10" rx=".6" /></Svg>;
