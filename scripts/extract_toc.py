import fitz

pdf_path = r'I:\My Drive\Books\from kindle\_OceanofPDF.com_The_Great_Mental_Models__General_Thinking_-_Shane_Parrish.pdf'
doc = fitz.open(pdf_path)

# Extract TOC pages (8-10 typically)
for i in range(7, 15):
    page = doc[i]
    text = page.get_text()
    print(f'=== Page {i+1} ===')
    print(text)
    print()

doc.close()
