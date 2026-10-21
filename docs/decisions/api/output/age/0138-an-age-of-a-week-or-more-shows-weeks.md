# ADR-0138. An age of a week or more shows weeks

- Date: 2026-10-22
- Status: accepted
- Supersedes: ADR-0108, which it amends
- Missed: ADR-0108 stopped at days although a week is a fixed seven days, so a column of older ages grew to three or four characters and disagreed with tools that count weeks.

## Context

ADR-0108 gave `Age` a compact text form in the largest whole unit, with the
day as the largest unit. A listing of older entries therefore showed ages
such as `20d`, `45d` or `400d`. An adopting tool replaced its own age
formatter with `Age` in its listing. That formatter also used weeks,
because a week is always seven days and a reader of a column compares rows
with each other. The tool's other commands still use its formatter, so its
listing and its status view printed different text for the same span.

A week has a fixed length in seconds, as a day does. The reason that
ADR-0108 gave against months and years, that their length depends on a
calendar, does not apply to it.

## Decision

- The compact age of seven days or more is the number of whole weeks, such
  as `1w` for 7 to 13 days, `2w` for 14 days and `57w` for 400 days. Below
  seven days the compact age is unchanged.
- The week is the largest unit. Months and years stay out for the reason
  that ADR-0108 gives.
- The text form changes in the text table, in a stream's text lines and in
  `Age`'s `Display`. JSON, the schema and the usage record keep the RFC 3339
  time.
- crumbkit adds no public option, constant or setting for this.

## Consequences

A column of ages up to 999 weeks, which is about 19 years, is at most four
characters wide, and an age under 100 weeks takes at most three characters.
Every tool that prints an `Age` shows weeks from seven days on, and a
golden file that holds an age of seven days or more changes with this
record. A tool that wants days for older ages formats the age itself from
`Age::elapsed`.

Considered: a setting that chooses the largest unit, which adds a setting
for one tool's output; keeping days and leaving the adopting tool's other
commands to switch, which keeps three- and four-character day counts in
every tool's columns.
