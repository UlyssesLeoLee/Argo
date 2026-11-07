"""Optional Playwright smoke: require a browser environment permitting localhost URLs.
Usage: python tests/e2e_argo_ui.py  (PLAYWRIGHT_URL must be set)
"""
import os
from playwright.sync_api import sync_playwright

url=os.getenv('ARGO_E2E_URL')
if not url:
    raise SystemExit('SKIP: set ARGO_E2E_URL to http://127.0.0.1:8899/?demo=1')
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True)
    page=browser.new_page(viewport={'width':1440,'height':960})
    errors=[]
    page.on('pageerror',lambda err: errors.append(str(err)))
    page.goto(url,wait_until='domcontentloaded')
    assert page.locator('.agent-tab').count()==7
    page.locator('[data-agent="orca"]').click()
    assert page.locator('#or').is_visible()
    assert page.locator('#mw').count()==0
    page.locator('[data-agent="multica"]').click()
    assert page.locator('#mw').is_visible()
    assert page.locator('#fileMachine').is_visible()
    assert page.locator('#executionMachine').is_visible()
    page.locator('[data-page="rag"]').click()
    assert page.locator('.rag-item').count()>=1
    page.locator('.rag-item').first.click()
    assert page.locator('#ragInspector').is_visible()
    assert not errors,errors
    print('OK: agent tabs, target machine, RAG list, inspector, no JS errors')
    browser.close()
