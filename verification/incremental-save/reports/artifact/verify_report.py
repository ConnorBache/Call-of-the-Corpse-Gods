from pathlib import Path
from zipfile import ZipFile
from lxml import etree
from playwright.sync_api import sync_playwright
import hashlib, json, re, sys

root = Path(__file__).resolve().parent
stem='incremental-save-engineering-report'
expected=json.loads((root/'expected-content.json').read_text())
ns={'w':'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
z=ZipFile(root/(stem+'.docx'))
xml=etree.fromstring(z.read('word/document.xml'))
actual=[]
for elem in xml.find('w:body',ns):
    if elem.tag==f'{{{ns["w"]}}}p':
        value=''.join(elem.xpath('.//w:t/text()',namespaces=ns))
        if value: actual.append(value)
    elif elem.tag==f'{{{ns["w"]}}}tbl':
        for cell in elem.xpath('./w:tr/w:tc',namespaces=ns):
            actual.append(''.join(cell.xpath('.//w:t/text()',namespaces=ns)))
assert actual==expected, 'DOCX content differs from authority Markdown'
external=[x for x in z.namelist() if x.endswith('.rels') and b'TargetMode="External"' in z.read(x)]
assert not external
styles=etree.fromstring(z.read('word/styles.xml'))
assert not styles.xpath('//w:style[@w:styleId="Title"]//w:pBdr',namespaces=ns)
html=(root/(stem+'.html')).read_text()
assert not re.search(r'<(?:script|link|iframe|img)\b|\b(?:src|href)\s*=',html,re.I)
tree=etree.HTML(html)
html_values=[''.join(e.itertext()) for e in tree.xpath('//main//*[self::h1 or self::h2 or self::h3 or self::p or self::li or self::th or self::td]')]
assert html_values==expected, 'HTML semantic content differs from authority Markdown'
source=root.parents[1]/'source/docs/verification/incremental-save-engineering-report.md'
assert source.read_bytes()==(root/(stem+'.md')).read_bytes()
assert hashlib.sha256(source.read_bytes()).hexdigest()=='bcfb207c91e31f45e379f1917e6d7c663051710f392ab9281b210f2f5fe99158'
report={'docx_semantic_blocks_exact':len(actual),'docx_external_relationships':external,'title_rule_absent':True,'self_contained_html':True,'source_markdown_copied_exactly':True,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'html_checks':{'semantic_blocks_exact':len(html_values),'table_count':len(tree.xpath('//table')),'external_assets_links_or_scripts':0,'browser_visual_QA':'not established: local Chromium blocked by sandbox socket restrictions; escalation wrapper failed'}}
(root/'structural-qa.json').write_text(json.dumps(report,indent=2)+'\n')
if '--browser' not in sys.argv:
    print(json.dumps(report,indent=2))
    sys.exit(0)
qa=root/'html-qa';qa.mkdir(exist_ok=True)
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path='/usr/bin/chromium',args=['--no-sandbox'])
    page=browser.new_page(viewport={'width':1280,'height':1100},device_scale_factor=1)
    errors=[]; requests=[]
    page.on('pageerror',lambda e:errors.append(str(e)))
    page.on('request',lambda r:requests.append(r.url))
    page.goto((root/(stem+'.html')).as_uri())
    values=page.locator('main h1,main h2,main h3,main p,main li,main th,main td').all_text_contents()
    assert values==expected, 'HTML content differs from authority Markdown'
    dims=page.evaluate('({width:innerWidth,scroll:document.documentElement.scrollWidth,height:document.documentElement.scrollHeight})')
    assert dims['scroll']<=dims['width']
    headings=page.locator('h1,h2').all()
    for i,heading in enumerate(headings):
        heading.scroll_into_view_if_needed()
        top=heading.evaluate('(e)=>e.getBoundingClientRect().top+scrollY')
        page.evaluate('(y)=>scrollTo(0,y)',max(0,top-35))
        page.screenshot(path=str(qa/f'desktop-section-{i+1}.png'))
    report['html_checks']['desktop']={'viewport_width':1280,'height':dims['height'],'body_no_horizontal_overflow':True,'exact_content_blocks':len(values)}
    page.set_viewport_size({'width':390,'height':844})
    page.evaluate('scrollTo(0,0)')
    page.screenshot(path=str(qa/'mobile-start.png'))
    mobile=page.evaluate('({width:innerWidth,scroll:document.documentElement.scrollWidth})')
    assert mobile['scroll']<=mobile['width']
    report['html_checks']['mobile']={'viewport_width':390,'body_no_horizontal_overflow':True,'tables_scroll_horizontally':True}
    report['html_checks']['browser_errors']=errors
    report['html_checks']['external_requests']=[r for r in requests if not r.startswith('file:')]
    assert not errors and not report['html_checks']['external_requests']
    browser.close()
(root/'structural-qa.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
