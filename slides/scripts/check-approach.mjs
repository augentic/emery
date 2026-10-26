import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const manifest = JSON.parse(readFileSync(join(root, 'content/approach.json'), 'utf8'))
const decks = [
  ['live', 'approach-live.md'],
  ['brief', 'approach-brief.md'],
]

function frontmatter(lines) {
  const values = {}

  for (const line of lines) {
    const match = /^([a-z][a-z0-9-]*):\s*(.*?)\s*$/i.exec(line)
    if (match)
      values[match[1]] = match[2]
  }

  return values
}

function parse(source, file) {
  const lines = source.split(/\r?\n/)
  const slides = []
  let line = 0

  while (line < lines.length) {
    while (line < lines.length && lines[line].trim() !== '---')
      line += 1

    if (line === lines.length)
      break

    const startsAt = line + 1
    line += 1
    const metadata = []

    while (line < lines.length && lines[line].trim() !== '---') {
      metadata.push(lines[line])
      line += 1
    }

    if (line === lines.length)
      throw new Error(`${file}:${startsAt}: frontmatter is not closed`)

    line += 1
    const body = []

    while (line < lines.length && lines[line].trim() !== '---') {
      body.push(lines[line])
      line += 1
    }

    slides.push({
      body: body.join('\n'),
      frontmatter: frontmatter(metadata),
      line: startsAt,
    })
  }

  return slides
}

function normalise(value) {
  return value
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/<style\b[\s\S]*?<\/style>/gi, ' ')
    .replace(/<script\b[\s\S]*?<\/script>/gi, ' ')
    .replace(/<br\s*\/?>/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#39;/gi, "'")
    .replace(/<[^>]+>/g, ' ')
    .replace(/[#*_`]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .toLocaleLowerCase('en-NZ')
}

const errors = []
const ids = manifest.map(({ id }) => id)
const bannedTerms = [
  /\bemery\b/i,
  /\bomnia\b/i,
  /\bclippy\.toml\b/i,
  /\bwasm32-wasip2\b/i,
]

if (new Set(ids).size !== ids.length)
  errors.push('content/approach.json contains a duplicate slide id')

for (const [kind, file] of decks) {
  const slides = parse(readFileSync(join(root, file), 'utf8'), file)
  const actual = slides.map(({ frontmatter: metadata }) => metadata.sync)

  if (slides.length !== manifest.length) {
    errors.push(
      `${file}: expected ${manifest.length} slides, found ${slides.length}`,
    )
  }

  for (let index = 0; index < manifest.length; index += 1) {
    const expected = manifest[index]
    const slide = slides[index]

    if (!slide) {
      errors.push(`${file}: missing slide ${index + 1} (${expected.id})`)
      continue
    }

    if (slide.frontmatter.sync !== expected.id) {
      errors.push(
        `${file}:${slide.line}: slide ${index + 1} must set sync: ${expected.id}; found ${slide.frontmatter.sync ?? '[missing]'}`,
      )
    }

    if (slide.frontmatter.title !== expected.title) {
      errors.push(
        `${file}:${slide.line}: ${expected.id} title must be "${expected.title}"; found "${slide.frontmatter.title ?? '[missing]'}"`,
      )
    }

    const visible = normalise(slide.body)

    for (const required of expected.required) {
      if (!visible.includes(normalise(required))) {
        errors.push(
          `${file}:${slide.line}: ${expected.id} is missing shared text "${required}"`,
        )
      }
    }

    for (const pattern of bannedTerms) {
      if (pattern.test(visible)) {
        errors.push(
          `${file}:${slide.line}: ${expected.id} contains internal term matching ${pattern}`,
        )
      }
    }

    if (kind === 'live') {
      const cardParagraphs = [
        ...slide.body.matchAll(/<div class="live-card\b[\s\S]*?<\/div>\s*<\/div>/g),
      ]
      for (const [block] of cardParagraphs) {
        for (const [, pText] of block.matchAll(/<p>([\s\S]*?)<\/p>/g)) {
          const words = normalise(pText).split(/\s+/).filter(Boolean)
          if (words.length > 15) {
            errors.push(
              `${file}:${slide.line}: ${expected.id} live-card <p> has ${words.length} words (max 15): "${normalise(pText)}"`,
            )
          }
        }
      }
    }
  }

  const duplicates = actual.filter(
    (id, index) => id && actual.indexOf(id) !== index,
  )

  if (duplicates.length > 0) {
    errors.push(
      `${file}: duplicate sync ids: ${[...new Set(duplicates)].join(', ')}`,
    )
  }

  const unknown = actual.filter(id => id && !ids.includes(id))
  if (unknown.length > 0) {
    errors.push(
      `${file}: unknown sync ids: ${[...new Set(unknown)].join(', ')}`,
    )
  }

  if (kind === 'brief' && actual.join('\0') !== ids.join('\0'))
    errors.push(`${file}: brief slide order no longer matches the live narrative`)
}

if (errors.length > 0) {
  console.error('Approach decks are out of sync:\n')
  for (const error of errors)
    console.error(`- ${error}`)
  process.exitCode = 1
}
else {
  console.log(`Approach decks are in sync (${manifest.length} slides).`)
}
