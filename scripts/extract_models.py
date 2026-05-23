import fitz
import sys

pdf_path = r'I:\My Drive\Books\from kindle\_OceanofPDF.com_The_Great_Mental_Models__General_Thinking_-_Shane_Parrish.pdf'
doc = fitz.open(pdf_path)
print(f'Pages: {len(doc)}')

# Extract first 20 pages to find the models
for i in range(min(20, len(doc))):
    page = doc[i]
    text = page.get_text()
    if text.strip():
        lines = text.strip().split('\n')[:5]
        print(f'Page {i+1}: {" | ".join(lines)}')

doc.close()
