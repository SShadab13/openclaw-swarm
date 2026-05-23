import fitz

pdf_path = r"I:\My Drive\Books\from kindle\_OceanofPDF.com_How_to_Read_a_Book_-_Charles_Van_Doren.pdf"
doc = fitz.open(pdf_path)

# Extract key framework pages
key_pages = [49, 50, 51, 144, 145, 146, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 280, 281, 282]

print("=" * 70)
print("ADLER'S FRAMEWORK - KEY EXTRACTS")
print("=" * 70)

for p in key_pages:
    page = doc[p]
    text = page.get_text()
    # Clean and output
    clean = text.encode('ascii', 'ignore').decode('ascii')
    lines = [l.strip() for l in clean.split('\n') if l.strip() and len(l.strip()) > 30]
    if lines:
        print(f"\n--- PAGE {p+1} ---")
        for line in lines[:8]:  # First 8 substantial lines
            print(line[:100])

print("\n" + "=" * 70)
print("FRAMEWORK SUMMARY")
print("=" * 70)
print("""
ADLER'S 4 LEVELS OF READING:
1. Elementary - basic literacy
2. Inspectional - skimming to get the gist (time-limited)
3. Analytical - thorough understanding (chewing & digesting)
4. Syntopical - reading multiple books on same subject to construct
   a unified understanding not present in any single book

ADLER'S 4 QUESTIONS (for Analytical Reading):
1. WHAT IS THE BOOK ABOUT AS A WHOLE?
   -> Discover the leading theme and how author develops it orderly
   
2. WHAT IS BEING SAID IN DETAIL, AND HOW?
   -> Main ideas, assertions, arguments. How author supports them.
   
3. IS THE BOOK TRUE, IN WHOLE OR PART?
   -> Must understand before judging. Suspend judgment until understood.
   
4. WHAT OF IT?
   -> If book has given you knowledge, what is your responsibility?
   -> What follows? What is further implied or suggested?

THE 3 STAGES OF ANALYTICAL READING:
Stage 1: Structure (Rules 1-4) -> Answer Question 1
  1. Classify the book (practical vs theoretical)
  2. State unity in a single sentence or short paragraph
  3. Outline major parts and their relation to whole
  4. Define the problems author tried to solve

Stage 2: Interpretation (Rules 5-8) -> Answer Question 2
  5. Come to terms with author (interpret key words)
  6. Grasp leading propositions (most important sentences)
  7. Know the arguments (sequences of sentences giving reasons)
  8. Determine which problems solved and which not

Stage 3: Criticism (Rules 9-12) -> Answer Questions 3-4
  9. Do not begin criticizing until you understand (suspend judgment)
  10. Do not disagree disputatiously or contentiously
  11. Demonstrate difference of opinion by quoting or paraphrasing
  12. Identify the difference between knowledge and mere opinion

THE 5 STEPS OF SYNTOPICAL READING:
1. Create tentative bibliography of subject
2. Inspect all books (inspectional reading) - identify relevant passages
3. Bring authors to terms - establish common terminology across books
4. Define the issues - frame questions all authors answer (pro/con)
5. Analyze the discussion - examine multiple sides, stay detached

THE SYNTOPICON:
- A constructed index of Great Ideas across many books
- Shows how different authors treat same concepts
- Enables construction of unified understanding from many sources
""")

doc.close()
