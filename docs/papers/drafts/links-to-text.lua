-- Drop hyperlinks (PDF drafts keep link text only; the repo paths are in the text).
function Link(el) return el.content end
