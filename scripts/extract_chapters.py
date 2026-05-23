import fitz

pdf_path = r'I:\My Drive\Books\from kindle\_OceanofPDF.com_The_Great_Mental_Models__General_Thinking_-_Shane_Parrish.pdf'
doc = fitz.open(pdf_path)

# Extract pages 20-35 (first model chapters)
for i in range(19, 35):
    page = doc[i]
    text = page.get_text()
    print(f"\n=== Page {i+1} ===")
    print(text[:1000])

doc.close()
