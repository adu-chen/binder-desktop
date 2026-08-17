#!/usr/bin/env node

/**
 * 功能明细表台账生成器（DOC-SYNC-01 / D2）
 *
 * 读取 .governance/function-ledger.json 数据源，生成
 * docs/00_core/generated/binder-product-function-ledger.xlsx。
 * 不依赖第三方库：以 SpreadsheetML（inlineStr）手写 xlsx，
 * 压缩由系统 zip 完成。
 *
 * 数据源是唯一人工维护的机器源；xlsx 为自动生成物，禁止手工编辑。
 */

/** @GOV
 * codes: BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001
 * type: RB
 * chain: PCG-GENERATE
 * rules: BR-PCG-DATA-001, BR-PCG-STATE-001
 * boundary: in=FunctionLedger data source | out=generated binder-product-function-ledger.xlsx | effect=WS-function-ledger | does_not=mutate_function_ledger_source
 * term_ref: TERM-PCG-001, TERM-PCG-002
 */

import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(SCRIPT_DIR, '..');

const esc = (value) => String(value ?? '').replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&apos;');

const STATUS_STYLES = {
  '待审计 · EP': '3',
  '已实现 · 待核验 · EP': '4',
  '规划 · V1+': '5',
  '明确不提供 · V1': '6',
  '概念 · VNext': '7',
};

const COLUMNS = ['id', 'l1', 'l2', 'l3', 'l4', 'l5', 'detail', 'status', 'acceptance', 'note'];
const HEADERS = ['功能 ID', 'L1 全屏页面', 'L2 UI 分区', 'L3 功能域', 'L4 Feature', 'L5 原子行为', '功能细节与边界', '当前状态', '验收摘要', '备注 / 说明'];

function buildSheet(ledger) {
  const totalRows = ledger.rows.length + 4; // 1 标题 + 1 说明 + 1 状态说明 + 1 表头
  const lastRow = totalRows;
  const colLetters = COLUMNS.map((_, i) => String.fromCharCode(65 + i));
  const cell = (ref, value, style = '') => `<c r="${ref}"${style ? ` s="${style}"` : ''} t="inlineStr"><is><t>${esc(value)}</t></is></c>`;

  let rows = '';
  rows += `<row r="1" ht="28" customHeight="1">${cell('A1', ledger.title, '1')}</row>`;
  rows += `<row r="2" ht="27" customHeight="1">${cell('A2', ledger.description, '1')}</row>`;
  rows += `<row r="3" ht="46" customHeight="1">${cell('A3', '状态说明：状态为文档盘点口径，不把「已实现」写成「已验收」。「已实现 · 待核验 · EP」表示 UI 入口与代码已实现，仍待测试/验证证据更新；其他状态需以测试/验证证据更新。H 列底色 = 状态分区。', '1')}</row>`;
  rows += `<row r="4" ht="34" customHeight="1">${HEADERS.map((h, i) => cell(`${colLetters[i]}4`, h, '2')).join('')}</row>`;

  ledger.rows.forEach((row, idx) => {
    const r = idx + 5;
    rows += `<row r="${r}" ht="28" customHeight="1">${COLUMNS.map((key, i) => {
      const style = key === 'status' ? (STATUS_STYLES[row.status] ?? '') : '';
      return cell(`${colLetters[i]}${r}`, row[key], style);
    }).join('')}</row>`;
  });

  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetPr><outlinePr summaryBelow="1" summaryRight="1" /><pageSetUpPr /></sheetPr><dimension ref="A1:J${lastRow}" /><sheetViews><sheetView workbookViewId="0"><pane ySplit="4" topLeftCell="A5" activePane="bottomLeft" state="frozen" /><selection pane="bottomLeft" activeCell="A5" sqref="A5" /></sheetView></sheetViews><sheetFormatPr baseColWidth="8" defaultRowHeight="15" /><cols><col width="16" customWidth="1" min="1" max="1" /><col width="14" customWidth="1" min="2" max="2" /><col width="16" customWidth="1" min="3" max="3" /><col width="17" customWidth="1" min="4" max="4" /><col width="19" customWidth="1" min="5" max="5" /><col width="24" customWidth="1" min="6" max="6" /><col width="43" customWidth="1" min="7" max="7" /><col width="21" customWidth="1" min="8" max="8" /><col width="37" customWidth="1" min="9" max="9" /><col width="42" customWidth="1" min="10" max="10" /></cols><sheetData>${rows}</sheetData><autoFilter ref="A4:J${lastRow}" /></worksheet>`;
}

function buildStyles() {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="2"><font><sz val="11" /><name val="PingFang SC" /></font><font><b /><sz val="11" /><name val="PingFang SC" /></font></fonts><fills count="7"><fill><patternFill patternType="none" /></fill><fill><patternFill patternType="gray125" /></fill><fill><patternFill patternType="solid"><fgColor rgb="FFDDEBF7" /></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFFFF2CC" /></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFE2EFDA" /></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFD9D9D9" /></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFF8CBAD" /></patternFill></fill></fills><borders count="2"><border><left style="thin"><color rgb="FFD0D7E3" /></left><right style="thin"><color rgb="FFD0D7E3" /></right><top style="thin"><color rgb="FFD0D7E3" /></top><bottom style="thin"><color rgb="FFD0D7E3" /></bottom><diagonal /></border><border><left style="thin"><color rgb="FFB7C4D8" /></left><right style="thin"><color rgb="FFB7C4D8" /></right><top style="thin"><color rgb="FFB7C4D8" /></top><bottom style="thin"><color rgb="FFB7C4D8" /></bottom><diagonal /></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" /></cellStyleXfs><cellXfs count="8"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" /><xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1" /><xf numFmtId="0" fontId="1" fillId="2" borderId="2" xfId="0" applyFont="1" applyFill="1" applyBorder="1" /><xf numFmtId="0" fontId="0" fillId="3" borderId="1" xfId="0" applyFill="1" applyBorder="1" /><xf numFmtId="0" fontId="0" fillId="4" borderId="1" xfId="0" applyFill="1" applyBorder="1" /><xf numFmtId="0" fontId="0" fillId="5" borderId="1" xfId="0" applyFill="1" applyBorder="1" /><xf numFmtId="0" fontId="0" fillId="6" borderId="1" xfId="0" applyFill="1" applyBorder="1" /><xf numFmtId="0" fontId="0" fillId="2" borderId="1" xfId="0" applyFill="1" applyBorder="1" /></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0" /></cellStyles></styleSheet>`;
}

function buildWorkbook() {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="功能明细表" sheetId="1" r:id="rId1" /></sheets></workbook>`;
}

function buildCoreProps() {
  const now = new Date().toISOString();
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:creator>Binder 治理工具</dc:creator><dc:title>Binder 产品功能明细表</dc:title><dcterms:created xsi:type="dcterms:W3CDTF">${now}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">${now}</dcterms:modified></cp:coreProperties>`;
}

function buildAppProps() {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>Binder product capability tools</Application><DocSecurity>0</DocSecurity><ScaleCrop>false</ScaleCrop><HeadingPairs /><TitlesOfParts /><Company /><LinksUpToDate>false</LinksUpToDate><SharedDoc>false</SharedDoc><HyperlinksChanged>false</HyperlinksChanged><AppVersion>1.0</AppVersion></Properties>`;
}

function main() {
  const ledgerPath = join(ROOT, '.governance/function-ledger.json');
  const ledger = JSON.parse(readFileSync(ledgerPath, 'utf8'));
  if (!ledger.rows?.length) throw new Error('功能明细台账数据源为空');
  const statuses = new Set(ledger.rows.map((r) => r.status));
  for (const status of statuses) {
    if (!(status in STATUS_STYLES)) throw new Error(`未登记的台账状态: ${status}`);
  }

  const outputDir = join(ROOT, 'docs/00_core/generated');
  mkdirSync(outputDir, { recursive: true });
  const work = mkdtempSync(join(tmpdir(), 'ledger-'));
  try {
    mkdirSync(join(work, '_rels'), { recursive: true });
    mkdirSync(join(work, 'xl/_rels'), { recursive: true });
    mkdirSync(join(work, 'xl/worksheets'), { recursive: true });
    mkdirSync(join(work, 'docProps'), { recursive: true });
    writeFileSync(join(work, '[Content_Types].xml'), `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml" /><Default Extension="xml" ContentType="application/xml" /><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml" /><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml" /><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml" /><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml" /><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml" /></Types>`);
    writeFileSync(join(work, '_rels/.rels'), `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml" /><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml" /><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml" /></Relationships>`);
    writeFileSync(join(work, 'xl/workbook.xml'), buildWorkbook());
    writeFileSync(join(work, 'xl/styles.xml'), buildStyles());
    writeFileSync(join(work, 'xl/worksheets/sheet1.xml'), buildSheet(ledger));
    writeFileSync(join(work, 'xl/_rels/workbook.xml.rels'), `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml" /><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml" /></Relationships>`);
    writeFileSync(join(work, 'docProps/core.xml'), buildCoreProps());
    writeFileSync(join(work, 'docProps/app.xml'), buildAppProps());

    const target = join(outputDir, 'binder-product-function-ledger.xlsx');
    execSync(`cd ${JSON.stringify(work)} && rm -f ${JSON.stringify(target)} && zip -r -q ${JSON.stringify(target)} '[Content_Types].xml' _rels docProps xl`, { stdio: 'inherit' });

    const counts = Object.fromEntries([...statuses].map((status) => [status, ledger.rows.filter((r) => r.status === status).length]));
    process.stdout.write(`${JSON.stringify({ status: 'passed', output: 'docs/00_core/generated/binder-product-function-ledger.xlsx', rows: ledger.rows.length, statuses: counts }, null, 2)}\n`);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

main();
