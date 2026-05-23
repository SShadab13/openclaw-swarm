import fitz

pdf_path = r'I:\My Drive\Books\from kindle\_OceanofPDF.com_The_Great_Mental_Models__General_Thinking_-_Shane_Parrish.pdf'
doc = fitz.open(pdf_path)

# Extract definitions for key models
models_to_extract = [
    "First Principles Thinking",
    "Occam's Razor", 
    "The Map is not the Territory",
    "Second-Order Thinking",
    "Inversion",
    "Circle of Competence",
    "Falsifiability",
    "Probabilistic Thinking"
]

for model_name in models_to_extract:
    found = False
    for i in range(len(doc)):
        page = doc[i]
        text = page.get_text()
        if model_name in text:
            print(f"\n=== {model_name} (Page {i+1}) ===")
            # Get surrounding context
            start = max(0, text.find(model_name) - 100)
            end = min(len(text), text.find(model_name) + 800)
            print(text[start:end])
            found = True
            break
    if not found:
        print(f"\n=== {model_name} NOT FOUND ===")

doc.close()
