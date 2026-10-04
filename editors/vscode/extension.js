// Orhunca VS Code eklentisi: `orhunca dil-sunucusu` ile konuşan LSP istemcisi.
'use strict';
const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let istemci;

function derleyici() {
  return vscode.workspace.getConfiguration('orhunca').get('derleyiciYolu') || 'orhunca';
}

function terminalde(komut) {
  const t = vscode.window.terminals.find(x => x.name === 'Orhunca') || vscode.window.createTerminal('Orhunca');
  t.show();
  t.sendText(komut);
}

async function activate(baglam) {
  istemci = new LanguageClient(
    'orhunca',
    'Orhunca',
    { command: derleyici(), args: ['dil-sunucusu'] },
    { documentSelector: [{ scheme: 'file', language: 'orhunca' }, { scheme: 'untitled', language: 'orhunca' }] }
  );
  baglam.subscriptions.push(
    vscode.commands.registerCommand('orhunca.calistir', async () => {
      const e = vscode.window.activeTextEditor;
      if (!e) return;
      await e.document.save();
      terminalde(`${derleyici()} çalıştır "${e.document.fileName}"`);
    }),
    vscode.commands.registerCommand('orhunca.denetle', async () => {
      const e = vscode.window.activeTextEditor;
      if (!e) return;
      await e.document.save();
      terminalde(`${derleyici()} denetle "${e.document.fileName}"`);
    }),
    vscode.commands.registerCommand('orhunca.studyo', () => terminalde(`${derleyici()} stüdyo`))
  );
  try {
    await istemci.start();
  } catch (hata) {
    vscode.window.showErrorMessage(
      `Orhunca dil sunucusu başlatılamadı (${hata.message}). "orhunca" komutunun kurulu olduğundan emin olun ya da Ayarlar'da orhunca.derleyiciYolu'nu belirtin.`
    );
  }
}

function deactivate() {
  return istemci ? istemci.stop() : undefined;
}

module.exports = { activate, deactivate };
