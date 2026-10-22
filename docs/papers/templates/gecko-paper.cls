% gecko-paper.cls: the GECKO journal look (page, fonts, headings, figures, citations).
% A paper says \documentclass{gecko-paper} and writes content only; see journal.tex.
% Requires XeLaTeX (fontspec, native pdf specials). Build with ./build_pdf.sh.
% It is a class, not a package, so the font fallback can run before article loads.
\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{gecko-paper}[2026/10/21 GECKO journal style]

% Bootstrap the default face when the optional Latin Modern fonts are missing.
\IfFileExists{lmroman10-regular.otf}{}{%
  \DeclareFontShape{TU}{lmr}{m}{n}{<->"Noto Sans"}{}%
  \DeclareFontShape{TU}{lmr}{bx}{n}{<->"Noto Sans:B"}{}%
  \DeclareFontShape{TU}{lmr}{m}{it}{<->"Noto Sans:I"}{}%
  \DeclareFontShape{TU}{lmr}{bx}{it}{<->"Noto Sans:BI"}{}%
}

\LoadClass[10pt,a4paper]{article}

\RequirePackage[margin=19mm]{geometry}
\RequirePackage[no-math]{fontspec}
\RequirePackage{xcolor,graphicx,multicol,caption,booktabs,titlesec,fancyhdr}

% Fonts. Medium (500) replaces bold (700) throughout.
\setmainfont{Noto Sans}[BoldFont={Noto Sans Medium},BoldItalicFont={Noto Sans Medium Italic}]
\setmonofont{DejaVu Sans Mono}
\newfontfamily\headingfont{Noto Sans Medium}[FakeBold=1.5]

% Colors (same tokens as web/src/styles.css).
\definecolor{ink}{HTML}{141414}
\definecolor{muted}{HTML}{707070}
\definecolor{accent}{HTML}{0066FF}
\definecolor{hair}{HTML}{E0E0E0}

% Compact reading sizes.
\renewcommand{\normalsize}{\fontsize{9}{12}\selectfont}
\renewcommand{\small}{\fontsize{8}{10.5}\selectfont}
\renewcommand{\footnotesize}{\fontsize{7}{9}\selectfont}
\renewcommand{\scriptsize}{\fontsize{6.5}{8}\selectfont}
\AtBeginDocument{\normalsize\color{ink}}
\setlength{\parindent}{0pt}
\setlength{\parskip}{6pt}
\setlength{\columnsep}{8mm}
\setlength{\multicolsep}{0pt}
\setlength{\emergencystretch}{1.5em}

% Headings.
\titleformat{\section}{\fontsize{9}{12}\selectfont\headingfont}{\thesection.}{0.45em}{}
\titlespacing*{\section}{0pt}{13pt}{3pt}
\titleformat{\subsection}{\fontsize{9}{12}\selectfont\headingfont}{\thesubsection}{0.45em}{}
\titlespacing*{\subsection}{0pt}{9pt}{2pt}

% Captions.
\captionsetup{font={small,color=muted},labelfont={bf,color=ink},justification=raggedright,singlelinecheck=false,skip=6pt}

% Footer: \paperfooter{left}{right}; the page number is appended on the right.
\pagestyle{fancy}
\fancyhf{}
\renewcommand{\headrulewidth}{0pt}
\renewcommand{\footrulewidth}{0.3pt}
\newcommand{\paperfooter}[2]{%
  \fancyfoot[L]{\scriptsize\color{muted}#1}%
  \fancyfoot[R]{\scriptsize\color{muted}#2 · \thepage}}

% --- Front matter -------------------------------------------------------
% Set with \papertitle{..}, \papersubtitle{..}, \paperauthor{..}, \paperaffil{..},
% \papernotice{..}, then call \makepapertitle. Use \\ for line breaks.
\newcommand{\papertitle}[1]{\def\gp@title{#1}}
\newcommand{\papersubtitle}[1]{\def\gp@subtitle{#1}}
\newcommand{\paperauthor}[1]{\def\gp@author{#1}}
\newcommand{\paperaffil}[1]{\def\gp@affil{#1}}
\newcommand{\papernotice}[1]{\def\gp@notice{#1}}
\def\gp@title{}\def\gp@subtitle{}\def\gp@author{}\def\gp@affil{}\def\gp@notice{}
% PDF metadata: \paperpdfinfo{Title}{Author}{Subject}
\newcommand{\paperpdfinfo}[3]{%
  \AtBeginDocument{\special{pdf:docinfo << /Title (#1) /Author (#2) /Subject (#3) >>}}}
\newcommand{\makepapertitle}{%
  \begin{center}
  {\fontsize{20}{24}\selectfont\bfseries \gp@title\par}
  \ifx\gp@subtitle\@empty\else\vspace{10pt}{\fontsize{10.5}{14}\selectfont \gp@subtitle\par}\fi
  \vspace{12pt}
  {\normalsize \gp@author\par}
  {\small\color{muted} \gp@affil\par}
  \ifx\gp@notice\@empty\else\vspace{8pt}{\footnotesize\color{muted} \gp@notice\par}\fi
  \end{center}
  \vspace{12pt}}

% Full-width abstract with generous space around it.
\newenvironment{paperabstract}{%
  \begin{quote}\raggedright
  \noindent{\fontsize{9}{12}\selectfont\headingfont Abstract}\par\vspace{-3pt}}{%
  \end{quote}\vspace{18pt}}

% Two-column body (continuous). Wrap everything after the abstract, bibliography included.
\newenvironment{paperbody}{\begin{multicols}{2}\raggedright}{\end{multicols}}

% --- Figures, citations, bibliography ----------------------------------
% \paperfigure{name}{caption}: includes assets/name.pdf at column width.
\newcommand{\paperfigure}[2]{%
  \par\addvspace{9pt}\noindent\begin{minipage}{\linewidth}
  \includegraphics[width=\linewidth]{assets/#1.pdf}
  \captionof{figure}{#2}
  \end{minipage}\par\addvspace{9pt}}

% Clickable, compact citations without an extra PDF-link package.
% \papercite{key} in text; \citetarget{key} at the start of each \bibitem.
\newcommand{\papercite}[1]{%
  \leavevmode\begingroup\scriptsize\color{accent}%
  \special{pdf:bann << /Type /Annot /Subtype /Link /Border [0 0 0] /A << /S /GoTo /D (cite.#1) >> >>}%
  \cite{#1}%
  \special{pdf:eann}\endgroup}
\newcommand{\citetarget}[1]{%
  \special{pdf:dest (cite.#1) [@thispage /XYZ @xpos @ypos null]}}

% Put right after \begin{thebibliography}{9}; \renewcommand{\refname}{..} sets the heading.
\newcommand{\paperbibstyle}{\small\raggedright\setlength{\itemsep}{6pt}}

% A booktabs table with a caption that stays in one piece.
% \begin{papertable}{caption} \begin{tabular*}... \end{tabular*} \end{papertable}
\newenvironment{papertable}[1]{%
  \begin{minipage}{\linewidth}\captionof{table}{#1}\small}{\end{minipage}}

% Small monospaced code block (verbatim: keep lines under ~48 characters).
\RequirePackage{fancyvrb}
\DefineVerbatimEnvironment{papercode}{Verbatim}{fontsize=\footnotesize,xleftmargin=0pt}

\endinput
