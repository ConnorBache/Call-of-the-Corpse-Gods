from pathlib import Path
import hashlib
import html
import json
import re
import shutil
from docx import Document
from docx.shared import Inches, Pt, RGBColor
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.enum.table import WD_TABLE_ALIGNMENT, WD_CELL_VERTICAL_ALIGNMENT
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[1] / 'source/docs/verification/incremental-save-engineering-report.md'
STEM = 'incremental-save-engineering-report'
raw = SOURCE.read_text()

def plain(s):
    return re.sub(r'\*\*(.*?)\*\*|`([^`]+)`', lambda m: m.group(1) if m.group(1) is not None else m.group(2), s)

def inline_html(s):
    chunks = re.split(r'(\*\*.*?\*\*|`[^`]+`)', s)
    return ''.join('<strong>'+html.escape(c[2:-2])+'</strong>' if c.startswith('**') else '<code>'+html.escape(c[1:-1])+'</code>' if c.startswith('`') else html.escape(c) for c in chunks)

def inline_docx(p, s, size=None, bold=False, color=None):
    for c in re.split(r'(\*\*.*?\*\*|`[^`]+`)', s):
        if not c:
            continue
        strong = c.startswith('**')
        code = c.startswith('`')
        t = c[2:-2] if strong else c[1:-1] if code else c
        r = p.add_run(t)
        if bold or strong:
            r.bold = True
        if code:
            r.font.name = 'DejaVu Sans Mono'
            r.font.size = Pt(size or 9.25)
        elif size:
            r.font.size = Pt(size)
        if color:
            r.font.color.rgb = RGBColor.from_string(color)

def blocks(lines):
    out, i = [], 0
    while i < len(lines):
        line = lines[i].strip()
        if not line:
            i += 1
            continue
        if line.startswith('#'):
            h = re.match(r'^(#+)\s+(.*)$',line)
            out.append(('heading',len(h[1]),h[2])); i += 1
        elif line.startswith('|'):
            rows = []
            while i < len(lines) and lines[i].strip().startswith('|'):
                row = [c.strip() for c in lines[i].strip().strip('|').split('|')]
                if not all(re.fullmatch(r':?-+:?',c) for c in row):
                    rows.append(row)
                i += 1
            out.append(('table',rows))
        elif line.startswith('- '):
            out.append(('bullet',line[2:])); i += 1
        else:
            p = [line]; i += 1
            while i < len(lines) and lines[i].strip() and not lines[i].startswith(('#','|','- ')):
                p.append(lines[i].strip()); i += 1
            out.append(('paragraph',' '.join(p)))
    return out

parsed = blocks(raw.splitlines())
doc = Document()
sec = doc.sections[0]
sec.page_width, sec.page_height = Inches(8.5), Inches(11)
sec.top_margin = sec.bottom_margin = Inches(.65)
sec.left_margin = sec.right_margin = Inches(.65)
sec.footer_distance = Inches(.28)
styles = doc.styles
for name in ['Normal','Title','Subtitle','Heading 1','Heading 2','Heading 3','List Bullet']:
    st = styles[name]
    st.font.name = 'Arial'
    st.font.color.rgb = RGBColor(0,0,0)
    st.element.get_or_add_rPr().append(OxmlElement('w:rFonts')) if st.element.get_or_add_rPr().find(qn('w:rFonts')) is None else None
    fonts = st.element.get_or_add_rPr().find(qn('w:rFonts'))
    fonts.set(qn('w:ascii'),'Arial'); fonts.set(qn('w:hAnsi'),'Arial')
    for attr in ['w:asciiTheme','w:hAnsiTheme','w:eastAsiaTheme','w:cstheme']:
        fonts.attrib.pop(qn(attr),None)
    for border in st.element.xpath('.//w:pBdr'):
        border.getparent().remove(border)
    for key in ['w:themeColor','w:themeTint','w:themeShade']:
        color = st.element.get_or_add_rPr().find(qn('w:color'))
        if color is not None:
            color.attrib.pop(qn(key),None)
styles['Normal'].font.size = Pt(11)
styles['Normal'].paragraph_format.space_after = Pt(7)
styles['Normal'].paragraph_format.line_spacing = 1.08
styles['Normal'].paragraph_format.widow_control = True
styles['Title'].font.size = Pt(24)
styles['Title'].font.bold = True
styles['Title'].paragraph_format.space_after = Pt(10)
for name, size, before, after in [('Heading 1',15,15,7),('Heading 2',12,12,6),('Heading 3',11,10,5)]:
    styles[name].font.size = Pt(size)
    styles[name].font.bold = True
    styles[name].paragraph_format.space_before = Pt(before)
    styles[name].paragraph_format.space_after = Pt(after)
    styles[name].paragraph_format.keep_with_next = True
styles['List Bullet'].paragraph_format.left_indent = Inches(.16)
styles['List Bullet'].paragraph_format.first_line_indent = Inches(-.16)
styles['List Bullet'].paragraph_format.space_after = Pt(7)
doc.core_properties.title = plain(parsed[0][2])
doc.core_properties.subject = 'Incremental save engineering findings and verification evidence'
doc.core_properties.author = ''
doc.core_properties.last_modified_by = ''
doc.core_properties.comments = ''

footer = sec.footer.paragraphs[0]
footer.alignment = WD_ALIGN_PARAGRAPH.RIGHT
footer.paragraph_format.space_after = Pt(0)
run = footer.add_run()
run.font.name = 'Arial'; run.font.size = Pt(9)
fld = OxmlElement('w:fldSimple'); fld.set(qn('w:instr'),'PAGE'); run._r.addnext(fld)

html_parts = []
expected_text = []
table_i = 0
widths_all = [
    [1.70,1.18,1.50,1.18,1.64],
    [1.65,1.02,1.02,1.02,1.23,1.26],
    [2.35,2.40,2.45],
    [1.70,1.66,1.96,1.88],
    [.68,2.13,1.16,1.94,1.29],
    [2.25,4.95],
]
for b in parsed:
    kind = b[0]
    if kind == 'heading':
        level, value = b[1:]
        p = doc.add_paragraph(style='Title' if level == 1 else 'Heading '+str(level-1))
        inline_docx(p,value)
        html_parts.append(f'<h{level}>{inline_html(value)}</h{level}>')
        expected_text.append(plain(value))
    elif kind in ['paragraph','bullet']:
        value = b[1]
        p = doc.add_paragraph(style='List Bullet' if kind == 'bullet' else None)
        inline_docx(p,value)
        html_parts.append(('<li>' if kind == 'bullet' else '<p>')+inline_html(value)+('</li>' if kind == 'bullet' else '</p>'))
        expected_text.append(plain(value))
    elif kind == 'table':
        rows = b[1]
        table = doc.add_table(rows=1, cols=len(rows[0]))
        table.alignment = WD_TABLE_ALIGNMENT.CENTER
        table.autofit = False
        widths = widths_all[table_i]
        for col,width in zip(table.columns,widths): col.width = Inches(width)
        tblpr = table._tbl.tblPr
        borders = OxmlElement('w:tblBorders')
        for edge in ['top','left','bottom','right','insideH','insideV']:
            e = OxmlElement('w:'+edge); e.set(qn('w:val'),'single'); e.set(qn('w:sz'),'4'); e.set(qn('w:color'),'D9D9D9'); borders.append(e)
        tblpr.append(borders)
        for ri, row in enumerate(rows):
            cells = table.rows[0].cells if ri == 0 else table.add_row().cells
            trpr = table.rows[ri]._tr.get_or_add_trPr()
            cant = OxmlElement('w:cantSplit'); trpr.append(cant)
            if ri == 0:
                rep = OxmlElement('w:tblHeader'); trpr.append(rep)
            for ci, (cell,value) in enumerate(zip(cells,row)):
                cell.width = Inches(widths[ci]); cell.vertical_alignment = WD_CELL_VERTICAL_ALIGNMENT.CENTER
                tcpr = cell._tc.get_or_add_tcPr()
                shd = OxmlElement('w:shd'); shd.set(qn('w:fill'),'17365D' if ri == 0 else ('F1F5F9' if ri%2 else 'FFFFFF')); tcpr.append(shd)
                mar = OxmlElement('w:tcMar')
                for side,val in [('top','80'),('bottom','80'),('left','80'),('right','80')]:
                    m = OxmlElement('w:'+side); m.set(qn('w:w'),val); m.set(qn('w:type'),'dxa'); mar.append(m)
                tcpr.append(mar)
                p = cell.paragraphs[0]
                p.paragraph_format.space_after = Pt(0)
                p.paragraph_format.space_before = Pt(0)
                p.paragraph_format.line_spacing = 1.03
                p.paragraph_format.keep_with_next = ri == 0
                numeric = table_i < 5 and ci > 0 and not (table_i == 4 and ci == 1)
                p.alignment = WD_ALIGN_PARAGRAPH.CENTER if numeric else WD_ALIGN_PARAGRAPH.LEFT
                inline_docx(p,value,size=9.5,bold=ri==0,color='FFFFFF' if ri==0 else '000000')
                expected_text.append(plain(value))
        p = doc.add_paragraph(); p.paragraph_format.space_after = Pt(2); p.paragraph_format.space_before = Pt(0); p.paragraph_format.line_spacing = .4
        p.add_run().font.size = Pt(3)
        html_parts.append('<div class="table-wrap"><table class="table-'+str(table_i+1)+'"><thead><tr>'+''.join('<th scope="col">'+inline_html(v)+'</th>' for v in rows[0])+'</tr></thead><tbody>'+''.join('<tr>'+''.join('<td>'+inline_html(v)+'</td>' for v in row)+'</tr>' for row in rows[1:])+'</tbody></table></div>')
        table_i += 1

docx_path = ROOT/(STEM+'.docx')
doc.save(docx_path)
body = '\n'.join(html_parts)
body = re.sub(r'((?:<li>.*?</li>\n?)+)',lambda m:'<ul>\n'+m[1]+'</ul>\n',body)
stylesheet = '''
:root{color-scheme:light}*{box-sizing:border-box}html{background:#f3f5f7;color:#15191d;font-family:Arial,Helvetica,sans-serif}body{margin:0}main{max-width:1024px;margin:32px auto;padding:48px 56px 60px;background:#fff;box-shadow:0 3px 18px #15253612}h1,h2,h3{color:#000;line-height:1.2;page-break-after:avoid}h1{font-size:31px;letter-spacing:-.5px;margin:0 0 16px}h2{font-size:23px;margin:34px 0 14px}h3{font-size:18px;margin:25px 0 12px}p,li{font-size:15.5px;line-height:1.58}p{margin:0 0 15px}ul{padding-left:22px;margin:0 0 20px}li{padding-left:3px;margin:0 0 11px}code{font-family:Consolas,'DejaVu Sans Mono',monospace;font-size:.88em;overflow-wrap:anywhere}strong{font-weight:700}.table-wrap{overflow-x:auto;margin:16px 0 21px}table{border-collapse:collapse;width:100%;font-size:13.5px;line-height:1.45;font-variant-numeric:tabular-nums}th,td{border:1px solid #d9d9d9;padding:10px 12px;vertical-align:middle}th{background:#17365d;color:#fff;text-align:left;font-size:13px}td:not(:first-child){text-align:right}tr:nth-child(odd) td{background:#f1f5f9}table:last-child td{overflow-wrap:normal}thead{display:table-header-group}tr{break-inside:avoid}a{color:inherit}@media(max-width:760px){main{margin:0;padding:26px 20px;box-shadow:none}h1{font-size:28px}h2{font-size:22px}p,li{font-size:15px}table{min-width:640px}th,td{padding:8px 10px}}@media print{html,body{background:#fff}main{max-width:none;margin:0;padding:0;box-shadow:none}p,li{font-size:10.5pt;line-height:1.35}h1{font-size:22pt}h2{font-size:15pt;margin-top:20pt}h3{font-size:12pt}table{font-size:9pt}th,td{padding:6pt}th{background:#e7edf5;color:#000}.table-wrap{overflow:visible}h1,h2,h3{break-after:avoid}p,li{orphans:2;widows:2}@page{size:Letter;margin:.65in}}
'''
stylesheet += '\n.table-5 td:nth-child(2),.table-6 td{text-align:left}.table-5 th:first-child,.table-5 td:first-child{text-align:center;white-space:nowrap}\n'
html_path = ROOT/(STEM+'.html')
html_path.write_text('<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>'+html.escape(plain(parsed[0][2]))+'</title><style>'+stylesheet+'</style></head><body><main>\n'+body+'\n</main></body></html>\n')
md_path = ROOT/(STEM+'.md')
shutil.copyfile(SOURCE,md_path)
(ROOT/'expected-content.json').write_text(json.dumps(expected_text,ensure_ascii=False,indent=2)+'\n')
manifest = {'source':str(SOURCE),'source_sha256':hashlib.sha256(raw.encode()).hexdigest(),'status':'final_authority_received_awaiting_visual_QA','table_count':table_i,'content_block_count':len(parsed),'outputs':{p.name:{'size_bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in [docx_path,html_path,md_path]}}
(ROOT/'artifact-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest,indent=2))
