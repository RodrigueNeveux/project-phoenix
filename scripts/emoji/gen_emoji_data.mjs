import fs from 'fs';

const [bundle, out] = process.argv.slice(2);
if (!bundle || !out) {
  console.error('usage: node gen_emoji_data.mjs <vnd-emoji.*.js> <emoji_data.rs>');
  process.exit(1);
}
const src = fs.readFileSync(bundle, 'utf8');
const start = src.indexOf("JSON.parse('{\"emojis\"");
if (start < 0) throw new Error('emoji database not found');
const open = start + 'JSON.parse('.length;
let end = open + 1;
while (src[end] !== "'") end += src[end] === '\\' ? 2 : 1;
const db = JSON.parse((0, eval)(src.slice(open, end + 1)));
const order = ['people', 'nature', 'food', 'activity', 'travel', 'objects', 'symbols', 'flags'];
const rust = (s) => JSON.stringify(s);
const rows = [];
order.forEach((cat, ci) => {
  const [from, to] = db.emojisByCategory[cat];
  for (let i = from; i < to; i++) {
    const e = db.emojis[i];
    const [name, ...aliases] = e.names;
    rows.push(`(${rust(e.surrogates)},${rust(name)},${rust(aliases.join(' '))},${ci},${e.hasDiversity === true})`);
  }
});
fs.writeFileSync(out, `#[rustfmt::skip]\npub static EMOJI: &[(&str, &str, &str, u8, bool)] = &[\n${rows.join(',\n')},\n];\n`);
console.log(`${rows.length} emoji`);
