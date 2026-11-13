# Versão, controles e FPS

Estas opções estão nos builds de desenvolvimento posteriores à primeira release
experimental. O APK da release `v0.1.0-alpha.1` não contém o backend integrado 1.8.9.

## Selecionar 1.8.9

Na página inicial, toque no botão **Version** e selecione **1.8.9**. Essa opção
seleciona o protocolo 47 para a conexão multiplayer. Informe o endereço do servidor
e use uma conta aceita por ele. Um perfil local serve para servidores que permitem
esse tipo de entrada; servidores autenticados exigem uma conta Minecraft Java.

O download inicial de recursos continua sendo o conjunto 26.3 usado pelo motor.
Selecionar 1.8.9 não instala uma JVM nem executa o cliente Java original. Mundos
locais continuam usando a simulação moderna. A compatibilidade multiplayer antiga
está em desenvolvimento; consulte [os testes e limitações](legacy-protocol.md).

## Botões de teclado na tela

Abra **Settings → Controls → Create WASD layout**. O preset oferece W, A, S, D,
Space, Shift, Ctrl, E, Q, T e botões de mouse para as ações correspondentes.
Use **Open Editor** para ajustar posições, tamanhos e comandos. A criação salva
um novo preset, preservando os layouts anteriores.

Os controles padrão também oferecem joystick, salto, agachamento, ataque e uso.
O teste automático de interação usa esse layout padrão. O preset WASD ainda
precisa de validação de toque separada.

## Contador de FPS

Em **Settings → Controls**, **Show FPS** ativa o contador nativo durante o jogo;
as opções próximas escolhem o canto e a exibição das mínimas de 1%.
Novos perfis têm o contador ativado. Uma preferência previamente salva para
desativá-lo é respeitada. Ele não aparece na tela de download ou de carregamento.

O número mostrado mede os frames do cliente. Ele não garante o mesmo desempenho
em outros aparelhos. Forge e OptiFine Java não são executados por este motor Rust.
