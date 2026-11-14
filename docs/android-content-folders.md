# Pastas de mods, texturas, shaders e controles

Ao abrir o aplicativo, o perfil ativo prepara suas pastas privadas e as pastas
externas pertencentes ao app:

```
Android/data/dev.taldoshawn.pojav/files/PojavRust/profile-0/
  resourcepacks/
  mods/
  shaderpacks/
  controls/
```

Outros perfis usam `profile-1`, `profile-2` etc. O Android pode restringir a
visualização de `Android/data` no gerenciador de arquivos. O gerenciador do
próprio launcher e o provedor de documentos do aplicativo também dão acesso
ao conteúdo instalado.

Arquivos novos nas pastas acima são copiados para o perfil ao tocar **Jogar**.
`mods` corresponde à pasta interna `horizon-mods`. Packs descompactados podem
ter subpastas. Arquivos já instalados são preservados; para substituir uma
versão, use a importação do gerenciador do aplicativo, que oferece a escolha
de substituição. Não é uma sincronização bidirecional: excluir a cópia externa
não exclui o conteúdo instalado.

Para usar uma pasta fora de `Android/data`, abra **Configurações → Controles →
Preparar / importar pasta**. No seletor Android, escolha ou crie uma pasta como
`Documentos/PojavRust`. O aplicativo cria `resourcepacks`, `mods`, `shaderpacks`
e `controls` dentro da pasta escolhida e importa seus arquivos novos. Após
adicionar arquivos com outro gerenciador, use essa opção novamente e selecione
a mesma pasta. A permissão é concedida pelo seletor de pastas do sistema.

- Texturas: resource packs ZIP ou pastas com `pack.mcmeta`; a compatibilidade
  dos nomes e formatos de assets depende do motor atual.
- Mods: conteúdo `.hmod` e mods nativos compatíveis com a API documentada.
- Shaders: somente os formatos aceitos pelo motor, sujeitos à GPU e ao renderer.
- Controles: presets JSON do cliente Rust, selecionados ou importados no editor.

Copiar um JAR Forge, OptiFine ou Fabric para `mods` não o transforma em um mod
nativo e não faz esse JAR executar. Esses carregadores Java não estão
implementados. As pastas não alteram a compatibilidade de versões do jogo.

A importação limita profundidade, quantidade de arquivos e tamanho de arquivo,
ignora links e arquivos ocultos, e publica cada cópia concluída sem substituir
um arquivo instalado. Contas e tokens permanecem no armazenamento privado.
