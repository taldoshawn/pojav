# Compilação do Pojav Rust

O workflow **Android APK** roda nos pushes em `main` e por acionamento manual.
Ele compila o código deste repositório; não reempacota um APK baixado de terceiros.
Os APKs são assinados e passam pela verificação de conteúdo do projeto antes do
envio aos artifacts. Recursos originais são baixados no aparelho; somente tabelas
geradas permitidas pelo empacotador ficam no APK público.

Os inputs temporários da preparação ficam em `.local/` no runner e nunca são
enviados ao Git ou aos artifacts. O cache guarda dependências Rust e resultados
de compilação, não contas, mundos, inputs do jogo ou material de assinatura.

O identificador Android é `dev.taldoshawn.pojav`, separado de outros launchers e
do Horizon original. A assinatura utiliza os secrets do repositório:

- `ANDROID_KEYSTORE_BASE64`
- `ANDROID_KEYSTORE_PASSWORD`
- `ANDROID_KEY_ALIAS`

Sem esses secrets, o build ainda gera um APK instalável com certificado temporário
de desenvolvimento. Esse artifact não garante atualização sobre uma instalação
de outra execução. Os APKs destinados a uma release devem ser assinados com a
chave permanente, fora do Git, antes da publicação.

O par chave/senha deve ser preservado fora do Git. O certificado precisa ser o
mesmo para atualizar instalações existentes; o número de versão aumenta em cada
execução do workflow. A chave privada não é incluída no APK ou no artifact.

Build manual, após preparar recursos com `tools/android_prepare.py`:

```sh
python3 tools/android_build.py --abi arm64-v8a --public --profile release \
  --app-id dev.taldoshawn.pojav --sdk "$ANDROID_HOME" --java-home "$JAVA_HOME" \
  --gradle "$(command -v gradle)" --ndk-version 28.2.13676358 \
  --build-tools-version 35.0.0 --dev-build local --dev-version-code 1000
```

Troque `arm64-v8a` por `armeabi-v7a`, `x86` ou `x86_64` quando necessário.
Verifique o arquivo `BUILD.json` e o SHA-256 junto do APK. Compilar e assinar não
substitui testar a instalação, a primeira inicialização, o multiplayer e o
desempenho num aparelho. O backend de protocolo 47 está integrado; a validação de gameplay no APK
ainda está em andamento.

## Assinar os artifacts com uma chave local permanente

Quando a integração GitHub não permite configurar secrets, o workflow gera
artifacts com assinatura temporária. Para distribuir uma release atualizável,
baixe os quatro artifacts do mesmo run e use `tools/sign_pojav_apks.py`:

```sh
python3 tools/sign_pojav_apks.py --artifacts .local/artifacts --output .local/release \
  --commit COMMIT_COMPLETO_DO_BUILD --version 0.1.0 \
  --keystore .local/signing/release.keystore --password-file .local/signing/password.txt \
  --build-tools "$ANDROID_HOME/build-tools/35.0.0"
```

O comando exige o mesmo commit nos quatro artifacts, verifica hashes e arquitetura,
assina sem alterar os arquivos de entrada e valida o conteúdo público novamente.
Publique somente os APKs resultantes, hashes e metadados públicos. Nunca envie a
chave ou a senha. Preserve a chave para futuras atualizações.

## Publicação sem enviar a chave privada ao Actions

`tools/apk_signature_patch.py` permite transportar a assinatura feita localmente:
referencia os dados comprimidos idênticos do artifact original e armazena somente
os cabeçalhos e blocos públicos alterados. Os hashes SHA-256 fixam tanto o APK
original quanto o APK final. O resultado deve ser idêntico, byte a byte, ao APK
assinado localmente. Esses arquivos não contêm chave privada nem senha.

O workflow `android-release.yml` baixa o run fixado, reconstrói os quatro APKs,
verifica assinatura, conteúdo e hashes e anexa os arquivos à release em rascunho.
Ele não publica automaticamente uma release nem aceita outro artifact silenciosamente.
Uma nova versão exige novos metadados e novas assinaturas locais.
