import { ExtensionContext, workspace } from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from 'vscode-languageclient/node';

import * as path from 'path';
import * as fs from 'fs';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    const config = workspace.getConfiguration('mecha');
    let serverPath = config.get<string>('serverPath');

    if (!serverPath || serverPath === 'lsp') {
        const bundledPath = context.asAbsolutePath(path.join('bin', 'lsp'));
        if (fs.existsSync(bundledPath)) {
            serverPath = bundledPath;
        } else {
            serverPath = 'lsp';
        }
    }

    const serverOptions: ServerOptions = {
        run: { command: serverPath, transport: TransportKind.stdio },
        debug: { command: serverPath, transport: TransportKind.stdio }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'mecha' }],
    };

    client = new LanguageClient(
        'mechaLsp',
        'Mecha LSP',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
