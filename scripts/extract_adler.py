import fitz  # PyMuPDF
import sys

pdf_path = r"I:\My Drive\Books\from kindle\_OceanofPDF.com_How_to_Read_a_Book_-_Charles_Van_Doren.pdf"
doc = fitz.open(pdf_path)

search_terms = [
    "four levels of reading",
    "four questions",
    "analytical reading",
    "syntopical reading",
    "synopticon",
    "what is the book about",
    "what is being said in detail",
    "is the book true",
    "what of it",
    "inspectional reading",
    "elementary reading",
    "coming to terms",
    "propositions",
    "arguments",
    "solutions"
]

print("=" * 70)
print("SEARCHING 'HOW TO READ A BOOK' FOR ADLER'S FRAMEWORK")
print(f"Total pages: {doc.page_count}")
print("=" * 70)

found_pages = set()
print("\n--- Page-by-page search for key terms ---")
for page_num in range(min(doc.page_count, 400)):
    page = doc[page_num]
    text = page.get_text().lower()
    
    hits = []
    for term in search_terms:
        if term.lower() in text:
            hits.append(term)
    
    if hits:
        found_pages.add(page_num)
        print(f"\nPage {page_num + 1}: {', '.join(hits[:3])}")
        full_text = page.get_text()
        for term in hits[:2]:
            idx = full_text.lower().find(term.lower())
            if idx >= 0:
                start = max(0, idx - 100)
                end = min(len(full_text), idx + 200)
                snippet = full_text[start:end].replace('\n', ' ')
                clean = snippet.encode('ascii', 'ignore').decode('ascii')
                print(f"   ...{clean}...")
                break

print(f"\n\nFound {len(found_pages)} relevant pages")
print(f"Page numbers: {sorted(found_pages)[:30]}")

doc.close()
