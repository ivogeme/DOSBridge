# DOSBridge

**Versão:** 0.5.0  
**Estado:** Experimental  
**Linguagem principal:** Rust

## Sobre o projeto

DOSBridge é um projeto experimental destinado a executar programas
desenvolvidos para o DOS em sistemas modernos.

O objetivo de longo prazo é desenvolver uma camada de compatibilidade
capaz de traduzir instruções de programas x86 de 16 bits para código
nativo da arquitetura do sistema hospedeiro durante a execução.

O projeto não pretende se limitar à emulação tradicional. A meta é
investigar uma abordagem baseada em tradução binária dinâmica
(Dynamic Binary Translation, ou DBT).

## Estado atual

A versão 0.5.0 utiliza o Unicorn Engine para executar instruções x86
e fornecer a base experimental para a compatibilidade com programas
DOS.

A implementação inclui componentes destinados a:

- Carregamento de executáveis DOS nos formatos MZ e COM.
- Preparação da memória para execução.
- Tratamento experimental de serviços DOS por meio da interrupção
  INT 21h.
- Integração com o Unicorn Engine.
- Desenvolvimento da infraestrutura inicial de compatibilidade.

Esses recursos não significam que todos os programas DOS sejam
compatíveis. O suporte depende das funcionalidades efetivamente
implementadas.

## Estado da compilação

Esta versão ainda não deve ser considerada estável.

Durante os testes no Termux para Android ARM64, `cargo check`
foi concluído com avisos, mas `cargo build` falhou durante a
ligação nativa do Unicorn Engine.

O erro observado foi:

    undefined symbol: __clear_cache

Portanto, a compilação completa para Android ARM64 ainda precisa
ser investigada e corrigida. A verificação bem-sucedida do código
Rust não significa que o executável final tenha sido produzido.

Não há garantia de que esta versão compile ou execute corretamente
em todos os sistemas.

## Objetivo da próxima versão

### 0.6.0: início da tradução binária dinâmica

O objetivo planejado para a versão 0.6.0 é começar a desenvolver
uma infraestrutura própria de tradução binária dinâmica.

As etapas previstas incluem:

1. Criar ou integrar um decodificador de instruções x86 de 16 bits.
2. Definir uma representação intermediária das instruções.
3. Implementar a tradução de um conjunto inicial de instruções
   x86 para código nativo x86-64.
4. Validar os resultados comparando a execução traduzida com a
   execução de referência no Unicorn Engine.
5. Investigar um cache de blocos de código traduzidos.
6. Expandir gradualmente a compatibilidade com programas DOS.

Esses itens são objetivos de desenvolvimento, não funcionalidades
já concluídas na versão 0.5.0.

## Arquiteturas

O primeiro alvo planejado para a tradução nativa é Linux x86-64.

O Android normalmente utiliza ARM64 nos dispositivos atuais.
Para executar código x86-64 nesse ambiente, será necessária uma
estratégia de tradução para ARM64 ou outra solução de compatibilidade.
A tradução x86-64 para código nativo x86-64, por si só, não resolve
essa diferença de arquitetura.

## Tecnologias

- Rust
- Cargo
- Unicorn Engine
- Arquitetura x86 de 16 bits
- DOS e seus serviços de compatibilidade
- Tradução binária dinâmica como objetivo de desenvolvimento

## Estrutura do projeto

    DOSBridge-0.5.0/
    ├── src/
    │   ├── main.rs
    │   ├── loader_mz.rs
    │   └── dos_api.rs
    ├── Cargo.toml
    ├── Cargo.lock
    └── README.md

A estrutura pode mudar conforme o projeto evoluir.

## Compilação

Com Rust e Cargo instalados, tente:

    cargo check

Esse comando verifica o código Rust sem necessariamente produzir
o executável final.

Para tentar compilar o programa:

    cargo build

A compilação depende das ferramentas nativas, das bibliotecas e
da compatibilidade do Unicorn Engine com o sistema hospedeiro.
Na configuração Android ARM64 testada, a ligação nativa ainda
apresentou o erro `undefined symbol: __clear_cache`.

## Contribuições e desenvolvimento

O DOSBridge está em desenvolvimento experimental. Problemas,
limitações e funcionalidades incompletas devem ser documentados
de forma transparente.

As metas futuras incluem ampliar a compatibilidade com programas
DOS e construir uma infraestrutura de tradução binária dinâmica,
com testes que permitam verificar a correção das instruções
traduzidas.

## Licença

A GPLV3
