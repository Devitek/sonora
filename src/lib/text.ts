// Joining transcript segments. Chinese and Japanese are written without spaces
// between words, so a space only goes after a segment that ends in a
// space-delimited script (Latin, Cyrillic, Hangul…).

// CJK symbols/punctuation, kana, Han ideographs and full-width forms
// (，。！？：；… 、 etc.).
const NO_SPACE_AFTER =
  /[\u3000-\u303f\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff00-\uffef]$/u;

/** Separator to put after `segment` before whatever is dictated next. */
export function segmentGap(segment: string): string {
  return NO_SPACE_AFTER.test(segment) ? "" : " ";
}

/** Join transcript segments with script-appropriate separators. */
export function joinSegments(segments: string[]): string {
  return segments
    .map((s) => s.trim())
    .filter(Boolean)
    .reduce((acc, s) => (acc ? acc + segmentGap(acc) + s : s), "");
}
