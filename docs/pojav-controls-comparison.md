# Comparação dos controles

Referência pública consultada: PojavLauncherTeam/PojavLauncher, branch
`v3_openjdk`, commit `b12ad048157b3aa255d078c235dd4571e1900309`.
Arquivos: `customcontrols/ControlData.java`, `buttons/ControlButton.java` e
`mouse/Touchpad.java`, em `app_pojavlauncher/src/main/java/net/kdt/pojavlaunch/`.
A referência usa LGPL-3.0; nenhum código Java foi incorporado nesta revisão.

O modelo de controles da referência inclui até quatro teclas por botão,
opacidade, tamanho, posição, alternância, visibilidade em menus/jogo,
deslizamento entre botões e passagem de movimento para a câmera.
O editor Rust possui campos correspondentes; isso não comprova equivalência
comportamental ou qualidade equivalente no aparelho.

## Verificação em andamento

- Teclas compartilhadas por dois dedos: emitir apenas uma transição de pressionar;
  soltar apenas após o último dedo. Corrigida emissão duplicada no código Rust.
- Alternância combinada com toque: ativar a trava de uma tecla já segurada não
  deve gerar outro pressionamento; retirar um dedo não deve liberar a trava.
- Cancelamento: liberar todas as teclas quando o gesto for cancelado.
- Persistência: conferir nome, tamanho, posição e comandos após reiniciar o APK.
- Mouse virtual: conferir clique, arrasto, sensibilidade e cursor personalizado.
- Teclado: conferir chat, caracteres acentuados e teclado físico.

O workflow Android executa os testes de controles customizados no host antes
finalizar a variante ARM64. Esses testes verificam lógica de entrada; testes
no APK e em aparelhos continuam necessários para avaliar latência e ergonomia.

## Painel lateral e HUD clássico

O preset **Controles → Usar HUD clássico Pojav** agora cria a grade de teclas
W/A/S/D, PRI/SEC, SHIFT, GUI, INV e SPACE, além da barra superior DEBUG,
CHAT, KEYBOARD, TAB, 3RD e MOUSE. Não substitui layouts personalizados existentes.
KEYBOARD abre o teclado do Android; dentro do mundo, abre também o campo de chat.

Selecionar um botão e abrir **Props** mostra um painel lateral, do lado oposto
à seleção, com uma lista rolável de nome, largura, altura, posição, quatro
atribuições, opacidade, cantos, borda, cores, alternância, passagem de toque,
deslizamento e visibilidade. Toque num valor, digite e confirme com Enter.
Medidas usam unidades da interface; cores aceitam oito dígitos AARRGGBB.
Arrastar a lista rola sem ativar o campo. Fechar durante uma entrada numérica
não confirmada descarta essa entrada. O salvamento de presets permanece no menu
Salvar do editor.

Essa revisão substitui o inspetor central dos botões. Ainda não representa uma
cópia integral do editor Pojav: o seletor de teclas e o menu de presets continuam
próprios; importação do JSON Pojav, gavetas de botões e expressões de posição
não foram implementadas. Equivalência visual completa não foi validada.

## Versão na tela inicial

A seta junto de **Jogar Minecraft 1.8.9 / 26.3** abre o seletor. A seleção é
persistida e agora aparece tanto no botão quanto no resumo do perfil.
1.8.9 seleciona o backend multiplayer de protocolo 47; mundos locais permanecem
26.3. O seletor não oferece versões sem implementação.
