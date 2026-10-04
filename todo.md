# Features to add

### 🟢 Difficulty: Low (The "Quality of Life" Polish)

#### 1. PDF Attachment & Auto-Renaming

Currently, you just store metadata. A "Diamond" editor manages the actual files.

* **The Feature:** Allow users to drag a PDF onto a row. The app links the file to the entry.
* **The "Diamond" Twist:** Offer to **auto-rename** and **move** the PDF based on the citation key (e.g., `~/Papers/Einstein1905.pdf`).
* **Rust Strategy:**
* Use `std::fs::rename`.
* Add a `file = {...}` field to your `biblatex` entry struct.
* Use `crate::open` to launch the system PDF viewer on click.



#### 2. Clipboard Watcher

* **The Feature:** If the user copies a DOI (`10.1038/...`) or a BibTeX string while browsing Chrome, MkBib shows a subtle "Toast" or pop-up: *"DOI detected. Add to library?"*
* **The "Diamond" Twist:** This saves the user from switching windows, clicking "Add," pasting, and clicking "Fetch."
* **Rust Strategy:**
* Crate: `arboard` (Clipboard) or `copypasta`.
* Run a background thread polling the clipboard every 1s.



---

### 🟡 Difficulty: Medium (The "Power User" Stuff)

#### 3. Real-Time CSL Preview (Citation Style Language)

* **The Feature:** Users edit BibTeX, but they want to know: *"How will this look in my bibliography?"*
* **The "Diamond" Twist:** A split pane at the bottom showing the rendered citation in **APA**, **IEEE**, or **Chicago** style.
* **Rust Strategy:**
* Crate: **`citationberg`** (The Rust CSL parser) or bindings to `citeproc`.
* This is huge because BibTeX raw data is ugly; the rendered output is what matters.



#### 4. Journal Abbreviation Manager

* **The Feature:** Scientists often need to switch between "Journal of The American Chemical Society" and "J. Am. Chem. Soc."
* **The "Diamond" Twist:** A one-click toggle to "Abbreviate All Journals" or "Expand All Journals" based on a built-in ISO 4 standard dictionary.
* **Rust Strategy:**
* You need a `HashMap<&str, &str>` lookup table (json file embedded in binary).
* Iterate over `model.bibliography`, swap the `journal` field, and refresh UI.



---

### 🔴 Difficulty: High (The "Magic" Features)

#### 5. Fuzzy Duplicate Detection

* **The Feature:** Your current duplicate scanner likely checks for exact Key or Title matches.
* **The "Diamond" Twist:** Detect that *"The theory of relativity"* and *"Theory of Relativity"* are the same paper. Detect that *"Smith, J."* and *"John Smith"* are likely the same.
* **Rust Strategy:**
* Crate: **`strsim`** (Levenshtein distance or Jaro-Winkler).
* Logic: If `similarity(title_a, title_b) > 0.90` AND `year == year`, flag as duplicate.
* UI: Show a "Merge Conflict" dialog where the user picks which fields to keep from A vs B.



#### 6. PDF-to-BibTeX (Grobid Integration)

* **The Feature:** User drags a PDF with *no* metadata into the window.
* **The "Diamond" Twist:** The app analyzes the text of the PDF, extracts the title/DOI automatically, fetches the metadata, and creates the entry. This feels like magic.
* **Rust Strategy:**
* **Hard Way:** Read PDF text (`pdf-extract`), regex for DOI.
* **Pro Way:** Connect to a local or public **Grobid** API (Machine Learning for parsing scientific papers). Send the PDF, get XML back.



---

### 🟣 Difficulty: God Tier (The "Commercial Killer")

#### 7. The "Graph" View

* **The Feature:** Instead of a list, show a **Node Graph**.
* **The "Diamond" Twist:** Nodes are papers. Edges are citations.
* "This paper cites these 5 papers."
* "This paper is cited by these 2 papers."


* **Rust Strategy:**
* Use **`OpenAlex`** API (which you just added!) to fetch "referenced_works".
* Render using a dedicated widget (custom drawing in GTK4 is tricky, but doable with `Cairo`).



**Recommendation:** Start with **#1 (PDF Renaming)** and **#3 (CSL Preview)**. Those two features alone make the app feel "premium."
