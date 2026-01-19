# msafara

`msafara` is a terminal-based viewer for multiple sequence alignments (MSAs).  It
provides a smooth interface to explore alignments directly from the
command line — especially useful when working over SSH or in headless
environments.
The binary name is `msafara`.

---

##  Installation

```bash
cargo install msafara
```

---

##  Quick Usage

Once installed, run:

```bash
msafara <my-alignment>
```
where `my-alignment` is a multiple alignment in Fasta or Stockholm format.

For help, run

```bash
msafara -h
```

Or press `?` while running to see key bindings.


---

## Features

- Zoomed-in and zoomed-out views of the alignment
- Consensus sequence display
- Sequence metrics such as ungapped length and similarity to consensus
- Ordering by metrics 
- Conservation indicators
- Color maps for nucleotides and amino acids
- Color themes
- Full keyboard control, no mouse required

Best results in a dark-themed terminal.

---

## More Info

- Source, releases, and test data:  
  [https://github.com/pmcarlton/msafara](https://github.com/pmcarlton/msafara)

- Platform-specific binaries (Linux, macOS) available on the [Releases](https://github.com/sib-swiss/msafara/releases) page.

---

## 📝 License

MIT
