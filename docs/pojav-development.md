# Desenvolvimento para Pojav Rust

O motor deste fork usa recursos e APIs do Horizon 26.3. A seleção 1.8.9 muda
somente a conexão multiplayer. Ela não transforma as APIs locais em APIs Forge.

## Controles e mouse

Em Settings → Controls, Create Pojav layout cria o preset de botões de teclado e
mouse. Open Editor permite selecionar um botão e alterar nome, posição, tamanho,
visibilidade e eventos de teclado/mouse. Salve um preset próprio para manter suas
alterações. Create WASD layout é uma alternativa com teclas W/A/S/D explícitas.
Virtual mouse speed ajusta o movimento do ponteiro; Virtual Cursor Scale muda
seu tamanho. MOUSE liga o ponteiro virtual. O teclado Android e teclados físicos
continuam usando a integração de entrada do jogo.

### Imagem do cursor

O cursor aceita uma textura PNG de um resource pack ativo:

`assets/minecraft/textures/gui/sprites/pojav/cursor.png`

Use transparência e coloque a ponta no canto superior esquerdo. A proporção da
imagem é preservada. Sem essa textura, o cursor vetorial padrão é usado.
O exemplo original em `examples/pojav-cursor/` pode ser compactado colocando
`pack.mcmeta` e `assets/` diretamente na raiz do ZIP. Importe pelo menu Resource
Pack, ative o pack e recarregue os recursos/reinicie o jogo. Esse recurso está no
código posterior ao APK alpha.2 e requer um APK recompilado.

## Texturas

Comece com o pack do cursor e adicione suas próprias imagens em
`assets/minecraft/textures/block/` ou `textures/item/`. Use nomes modernos, como
`stone.png`, mesmo na conexão 1.8.9: o motor converte os IDs antigos antes de
renderizar. Packs antigos 1.8.9 podem precisar de conversão de nomes e modelos.
Consulte [resource-packs.md](resource-packs.md) para formatos, animações e limitações.
Não redistribua os recursos originais do Minecraft junto do seu pack.

## Mods de conteúdo

Use o CLI deste mesmo checkout, evitando depender de uma API de outra revisão:

```sh
cargo install --path crates/cargo-horizon-mod --locked
cargo horizon-mod new meu_mod
cd meu_mod
cargo horizon-mod build
```

Consulte `cargo horizon-mod` para a sintaxe da revisão.
O manifesto `horizon-mod.toml`, assets e data formam o pacote `.hmod`.
[mods.md](mods.md) explica capacidades e instalação. Conteúdo novo em multiplayer
exige suporte do servidor; um servidor Java 1.8.9 comum não conhece blocos novos.

## Mods clientes em Rust

O SDK e exemplos estão em `crates/horizon-mod-sdk/`. O exemplo
`examples/coords-hud` desse crate mostra leitura de coordenadas, painel de HUD e
comandos locais. Leia seu manifesto de capacidades e seu `src/lib.rs` antes de
alterar o exemplo. O SDK, o empacotamento e o carregamento precisam corresponder
à revisão do jogo: compilar uma biblioteca Rust não basta para instalá-la.

Para alterações nativas no código do cliente, consulte [native-mods.md](native-mods.md)
e [horizon-native-mods.md](horizon-native-mods.md). Android requer compilação para
sua ABI com o NDK; o aparelho não compila esses mods sozinho. A disponibilidade
do serviço de compilação descrito no upstream precisa ser configurada para este
fork antes de ser prometida como serviço ao usuário.

Fabric, Forge, NeoForge e OptiFine Java não executam diretamente neste motor.
Não existe garantia de compatibilidade com todos os mods, packs ou servidores.
