//! The notices of the readers in the eight languages of the suite.
//!
//! A [`Diagnostic`] carries an English `message` with the details of the case (the number, the name) and a
//! stable `code`. This module gives the **headline of each code** in English, Spanish, German, French,
//! Italian, Japanese, Chinese and Portuguese of Portugal, so a program can show a notice in the language of
//! the person without parsing the English text. [`localize`] writes one notice as a line in a language.
//!
//! Every code of [`crate::codes::ALL`] has its eight headlines (a test fails when one is missing or empty),
//! so a new code cannot be added without its translations.

use crate::Diagnostic;

/// A language of the suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// English.
    En,
    /// Spanish.
    Es,
    /// German.
    De,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Japanese.
    Ja,
    /// Chinese.
    Zh,
    /// Portuguese of Portugal.
    Pt,
}

impl Language {
    /// The eight languages, in the order of the headlines of the table.
    pub const ALL: [Language; 8] = [
        Language::En,
        Language::Es,
        Language::De,
        Language::Fr,
        Language::It,
        Language::Ja,
        Language::Zh,
        Language::Pt,
    ];

    /// The short tag of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn tag(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Es => "es",
            Language::De => "de",
            Language::Fr => "fr",
            Language::It => "it",
            Language::Ja => "ja",
            Language::Zh => "zh",
            Language::Pt => "pt",
        }
    }

    /// The language of a tag such as `pt`, `pt-PT` or `zh_CN` (only the first two letters count, in any case).
    pub fn from_tag(tag: &str) -> Option<Language> {
        let lower = tag.trim().to_ascii_lowercase();
        let head = lower.get(..2)?;
        Language::ALL.into_iter().find(|l| l.tag() == head)
    }

    fn index(self) -> usize {
        Language::ALL.iter().position(|l| *l == self).unwrap_or(0)
    }

    /// The word "line" in the language, for the position of a notice.
    pub fn line_word(self) -> &'static str {
        [
            "line", "línea", "Zeile", "ligne", "riga", "行", "行", "linha",
        ][self.index()]
    }
}

/// The headline of a code in a language, or `None` for a code that does not exist.
pub fn headline(code: &str, language: Language) -> Option<&'static str> {
    TABLE
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[language.index()])
}

/// A notice as one line in a language: the headline of its code, the code and the line number. The English
/// `message` of the diagnostic keeps the details (names and numbers) for whoever wants them.
pub fn localize(diagnostic: &Diagnostic, language: Language) -> String {
    let text = headline(diagnostic.code, language).unwrap_or(diagnostic.code);
    format!(
        "{text} [{}] ({} {})",
        diagnostic.code,
        language.line_word(),
        diagnostic.line
    )
}

type Row = (&'static str, [&'static str; 8]);

/// The codes with their headlines in the order of [`Language::ALL`].
#[rustfmt::skip]
const TABLE: &[Row] = &[
    ("units-missing", ["The file does not declare its units; millimetres are assumed", "El fichero no declara sus unidades; se suponen milímetros", "Die Datei gibt keine Einheit an; Millimeter werden angenommen", "Le fichier ne déclare pas ses unités ; des millimètres sont supposés", "Il file non dichiara le unità; si assumono i millimetri", "ファイルに単位の宣言がありません。ミリメートルとみなします", "文件未声明单位，按毫米处理", "O ficheiro não declara as unidades; assumem-se milímetros"]),
    ("number-too-large", ["A number is too large to be read", "Un número es demasiado grande para leerlo", "Eine Zahl ist zu groß, um gelesen zu werden", "Un nombre est trop grand pour être lu", "Un numero è troppo grande per essere letto", "数値が大きすぎて読み取れません", "数字太大，无法读取", "Um número é demasiado grande para ser lido"]),
    ("coordinate-invalid", ["The digits are not a valid coordinate", "Los dígitos no son una coordenada válida", "Die Ziffern sind keine gültige Koordinate", "Les chiffres ne forment pas une coordonnée valide", "Le cifre non sono una coordinata valida", "数字が有効な座標ではありません", "这些数字不是有效的坐标", "Os dígitos não são uma coordenada válida"]),
    ("unknown-g-code", ["Unknown G code, ignored", "Código G desconocido, ignorado", "Unbekannter G-Code, ignoriert", "Code G inconnu, ignoré", "Codice G sconosciuto, ignorato", "未知の G コードを無視しました", "未知的 G 代码，已忽略", "Código G desconhecido, ignorado"]),
    ("unknown-m-code", ["Unknown M code, ignored", "Código M desconocido, ignorado", "Unbekannter M-Code, ignoriert", "Code M inconnu, ignoré", "Codice M sconosciuto, ignorato", "未知の M コードを無視しました", "未知的 M 代码，已忽略", "Código M desconhecido, ignorado"]),
    ("unknown-word", ["Unknown word, ignored", "Palabra desconocida, ignorada", "Unbekanntes Wort, ignoriert", "Mot inconnu, ignoré", "Parola sconosciuta, ignorata", "未知のワードを無視しました", "未知的字，已忽略", "Palavra desconhecida, ignorada"]),
    ("end-missing", ["The file does not end with its end command", "El fichero no termina con su orden de fin", "Die Datei endet nicht mit ihrem Endbefehl", "Le fichier ne se termine pas par sa commande de fin", "Il file non termina con il suo comando di fine", "ファイルが終了コマンドで終わっていません", "文件没有以结束命令结尾", "O ficheiro não termina com o seu comando de fim"]),
    ("incremental-unsupported", ["Incremental coordinates are not supported here", "Las coordenadas incrementales no se admiten aquí", "Inkrementelle Koordinaten werden hier nicht unterstützt", "Les coordonnées incrémentales ne sont pas prises en charge ici", "Le coordinate incrementali non sono supportate qui", "ここでは相対座標をサポートしていません", "此处不支持增量坐标", "As coordenadas incrementais não são suportadas aqui"]),
    ("deprecated-command", ["Deprecated command", "Orden obsoleta", "Veralteter Befehl", "Commande obsolète", "Comando obsoleto", "非推奨のコマンドです", "已弃用的命令", "Comando obsoleto"]),
    ("command-ignored", ["Command with no effect, ignored", "Orden sin efecto, ignorada", "Befehl ohne Wirkung, ignoriert", "Commande sans effet, ignorée", "Comando senza effetto, ignorato", "効果のないコマンドを無視しました", "无效果的命令，已忽略", "Comando sem efeito, ignorado"]),
    ("text-after-end", ["Text after the end of the program", "Texto después del fin del programa", "Text nach dem Programmende", "Texte après la fin du programme", "Testo dopo la fine del programma", "プログラム終了後にテキストがあります", "程序结束之后还有文本", "Texto depois do fim do programa"]),
    ("input-not-utf8", ["The bytes are not UTF-8; read as Latin-1", "Los bytes no son UTF-8; se leen como Latin-1", "Die Bytes sind kein UTF-8; als Latin-1 gelesen", "Les octets ne sont pas de l'UTF-8 ; lus comme du Latin-1", "I byte non sono UTF-8; letti come Latin-1", "UTF-8 ではないため Latin-1 として読み込みました", "字节不是 UTF-8，按 Latin-1 读取", "Os bytes não são UTF-8; lidos como Latin-1"]),
    ("too-many-diagnostics", ["Too many notices; the rest are not kept", "Demasiados avisos; el resto no se guarda", "Zu viele Hinweise; der Rest wird nicht gespeichert", "Trop d'avis ; les autres ne sont pas conservés", "Troppi avvisi; gli altri non vengono conservati", "通知が多すぎるため、残りは保存しません", "提示过多，其余不再保留", "Demasiados avisos; os restantes não são guardados"]),
    ("command-unsupported", ["Command not supported, ignored", "Orden no admitida, ignorada", "Befehl nicht unterstützt, ignoriert", "Commande non prise en charge, ignorée", "Comando non supportato, ignorato", "未対応のコマンドを無視しました", "不支持的命令，已忽略", "Comando não suportado, ignorado"]),
    ("unknown-command", ["Unknown command", "Orden desconocida", "Unbekannter Befehl", "Commande inconnue", "Comando sconosciuto", "未知のコマンド", "未知的命令", "Comando desconhecido"]),
    ("geometry-command-unsupported", ["A command that changes the geometry is not supported", "Una orden que cambia la geometría no se admite", "Ein Befehl, der die Geometrie ändert, wird nicht unterstützt", "Une commande qui change la géométrie n'est pas prise en charge", "Un comando che cambia la geometria non è supportato", "形状を変えるコマンドはサポートされていません", "不支持会改变几何形状的命令", "Um comando que altera a geometria não é suportado"]),
    ("unknown-header-line", ["Unknown header line", "Línea de cabecera desconocida", "Unbekannte Kopfzeile", "Ligne d'en-tête inconnue", "Riga di intestazione sconosciuta", "未知のヘッダー行", "未知的文件头行", "Linha de cabeçalho desconhecida"]),
    ("position-untracked", ["The position can no longer be followed", "Ya no se puede seguir la posición", "Die Position kann nicht mehr verfolgt werden", "La position ne peut plus être suivie", "La posizione non può più essere seguita", "位置を追跡できなくなりました", "无法再跟踪位置", "Já não é possível seguir a posição"]),
    ("extended-unclosed", ["An extended command is never closed", "Una orden extendida no se cierra nunca", "Ein erweiterter Befehl wird nie geschlossen", "Une commande étendue n'est jamais fermée", "Un comando esteso non viene mai chiuso", "拡張コマンドが閉じられていません", "扩展命令从未关闭", "Um comando estendido nunca é fechado"]),
    ("block-unclosed", ["A block is not closed by an asterisk", "Un bloque no se cierra con asterisco", "Ein Block wird nicht mit einem Stern beendet", "Un bloc n'est pas fermé par un astérisque", "Un blocco non è chiuso da un asterisco", "ブロックがアスタリスクで閉じられていません", "程序段没有以星号结束", "Um bloco não é fechado por um asterisco"]),
    ("region-unclosed", ["A region is never closed", "Una región no se cierra nunca", "Eine Region wird nie geschlossen", "Une région n'est jamais fermée", "Una regione non viene mai chiusa", "リージョンが閉じられていません", "区域从未关闭", "Uma região nunca é fechada"]),
    ("region-nested", ["A region inside a region", "Una región dentro de otra región", "Eine Region innerhalb einer Region", "Une région dans une région", "Una regione dentro una regione", "リージョンの中にリージョンがあります", "区域嵌套在区域内", "Uma região dentro de uma região"]),
    ("region-not-open", ["The end of a region without a start", "Fin de región sin inicio", "Ein Regionsende ohne Anfang", "Fin de région sans début", "Fine di regione senza inizio", "開始のないリージョン終了です", "区域结束前没有开始", "Fim de região sem início"]),
    ("macro-name-missing", ["An aperture macro has no name", "Una macro de apertura no tiene nombre", "Ein Blendenmakro hat keinen Namen", "Une macro d'ouverture n'a pas de nom", "Una macro di apertura non ha nome", "アパーチャマクロに名前がありません", "光圈宏没有名称", "Uma macro de abertura não tem nome"]),
    ("units-unknown", ["Unknown units", "Unidades desconocidas", "Unbekannte Einheiten", "Unités inconnues", "Unità sconosciute", "未知の単位", "未知的单位", "Unidades desconhecidas"]),
    ("polarity-unknown", ["Unknown polarity", "Polaridad desconocida", "Unbekannte Polarität", "Polarité inconnue", "Polarità sconosciuta", "未知の極性", "未知的极性", "Polaridade desconhecida"]),
    ("format-zero-omission-invalid", ["The zero omission must be leading or trailing", "La omisión de ceros debe ser delantera o trasera", "Die Nullunterdrückung muss führend oder nachgestellt sein", "L'omission des zéros doit être en tête ou en queue", "L'omissione degli zeri deve essere iniziale o finale", "ゼロ省略は先頭か末尾でなければなりません", "零省略必须是前导或尾随", "A omissão de zeros deve ser à frente ou atrás"]),
    ("format-trailing-deprecated", ["Omitting trailing zeros is deprecated", "Omitir los ceros finales está obsoleto", "Das Weglassen nachgestellter Nullen ist veraltet", "Omettre les zéros de fin est obsolète", "Omettere gli zeri finali è obsoleto", "末尾のゼロ省略は非推奨です", "省略尾随零已弃用", "Omitir os zeros finais está obsoleto"]),
    ("format-mode-invalid", ["The coordinate mode is invalid", "El modo de coordenadas no es válido", "Der Koordinatenmodus ist ungültig", "Le mode de coordonnées est invalide", "La modalità delle coordinate non è valida", "座標モードが無効です", "坐标模式无效", "O modo de coordenadas não é válido"]),
    ("format-malformed", ["The coordinate format is malformed", "El formato de coordenadas está mal formado", "Das Koordinatenformat ist fehlerhaft", "Le format de coordonnées est mal formé", "Il formato delle coordinate è malformato", "座標フォーマットが不正です", "坐标格式格式错误", "O formato de coordenadas está mal formado"]),
    ("format-axes-differ", ["X and Y use different formats", "X e Y usan formatos distintos", "X und Y verwenden unterschiedliche Formate", "X et Y utilisent des formats différents", "X e Y usano formati diversi", "X と Y で異なるフォーマットです", "X 和 Y 使用不同的格式", "X e Y usam formatos diferentes"]),
    ("format-digits-invalid", ["The digits of the format are invalid", "Los dígitos del formato no son válidos", "Die Stellenzahlen des Formats sind ungültig", "Les chiffres du format sont invalides", "Le cifre del formato non sono valide", "フォーマットの桁数が無効です", "格式的位数无效", "Os dígitos do formato não são válidos"]),
    ("coordinate-before-format", ["A coordinate comes before the format", "Una coordenada aparece antes del formato", "Eine Koordinate steht vor dem Format", "Une coordonnée précède le format", "Una coordinata precede il formato", "フォーマットより前に座標があります", "坐标出现在格式之前", "Uma coordenada aparece antes do formato"]),
    ("aperture-number-invalid", ["The aperture number is invalid", "El número de apertura no es válido", "Die Blendennummer ist ungültig", "Le numéro d'ouverture est invalide", "Il numero di apertura non è valido", "アパーチャ番号が無効です", "光圈编号无效", "O número de abertura não é válido"]),
    ("aperture-number-reserved", ["The aperture number is reserved", "El número de apertura está reservado", "Die Blendennummer ist reserviert", "Le numéro d'ouverture est réservé", "Il numero di apertura è riservato", "アパーチャ番号は予約されています", "光圈编号为保留编号", "O número de abertura está reservado"]),
    ("aperture-parameter-invalid", ["An aperture parameter is not a number", "Un parámetro de apertura no es un número", "Ein Blendenparameter ist keine Zahl", "Un paramètre d'ouverture n'est pas un nombre", "Un parametro di apertura non è un numero", "アパーチャのパラメータが数値ではありません", "光圈参数不是数字", "Um parâmetro de abertura não é um número"]),
    ("aperture-parameters-missing", ["An aperture has too few parameters", "Una apertura tiene muy pocos parámetros", "Eine Blende hat zu wenige Parameter", "Une ouverture a trop peu de paramètres", "Un'apertura ha troppi pochi parametri", "アパーチャのパラメータが足りません", "光圈参数太少", "Uma abertura tem parâmetros a menos"]),
    ("aperture-polygon-vertices", ["A polygon aperture has 3 to 12 vertices", "Una apertura poligonal tiene de 3 a 12 vértices", "Eine Polygonblende hat 3 bis 12 Ecken", "Une ouverture polygonale a de 3 à 12 sommets", "Un'apertura poligonale ha da 3 a 12 vertici", "多角形アパーチャの頂点は 3 から 12 です", "多边形光圈有 3 到 12 个顶点", "Uma abertura poligonal tem de 3 a 12 vértices"]),
    ("aperture-macro-undefined", ["The macro is not defined before its use", "La macro no está definida antes de usarla", "Das Makro ist vor der Verwendung nicht definiert", "La macro n'est pas définie avant son usage", "La macro non è definita prima dell'uso", "マクロが使用前に定義されていません", "宏在使用前未定义", "A macro não está definida antes de ser usada"]),
    ("aperture-template-missing", ["The aperture has no template", "La apertura no tiene plantilla", "Die Blende hat keine Vorlage", "L'ouverture n'a pas de modèle", "L'apertura non ha un modello", "アパーチャにテンプレートがありません", "光圈没有模板", "A abertura não tem modelo"]),
    ("aperture-size-negative", ["An aperture has a negative size", "Una apertura tiene un tamaño negativo", "Eine Blende hat eine negative Größe", "Une ouverture a une taille négative", "Un'apertura ha una dimensione negativa", "アパーチャのサイズが負です", "光圈尺寸为负", "Uma abertura tem um tamanho negativo"]),
    ("aperture-redefined", ["An aperture is defined twice; the last one wins", "Una apertura se define dos veces; vale la última", "Eine Blende ist doppelt definiert; die letzte gilt", "Une ouverture est définie deux fois ; la dernière l'emporte", "Un'apertura è definita due volte; vale l'ultima", "アパーチャが二重に定義されています。最後の定義を使います", "光圈被定义了两次，以最后一次为准", "Uma abertura é definida duas vezes; vale a última"]),
    ("aperture-not-selected", ["A draw or flash happens before an aperture is selected", "Se dibuja o destella antes de elegir una apertura", "Es wird gezeichnet oder geblitzt, bevor eine Blende gewählt ist", "On trace ou on flashe avant de choisir une ouverture", "Si disegna o si flasha prima di scegliere un'apertura", "アパーチャを選ぶ前に描画またはフラッシュしています", "在选择光圈之前就绘制或闪烁", "Desenha-se ou faz-se flash antes de escolher uma abertura"]),
    ("aperture-undefined", ["The aperture is not defined", "La apertura no está definida", "Die Blende ist nicht definiert", "L'ouverture n'est pas définie", "L'apertura non è definita", "アパーチャが定義されていません", "光圈未定义", "A abertura não está definida"]),
    ("step-repeat-invalid", ["The step and repeat is invalid", "El paso y repetición no es válido", "Das Step-and-Repeat ist ungültig", "Le pas et répétition est invalide", "Lo step and repeat non è valido", "ステップアンドリピートが無効です", "步进重复无效", "O passo e repetição não é válido"]),
    ("block-empty", ["A block has no letters and was ignored", "Un bloque sin letras se ignoró", "Ein Block ohne Buchstaben wurde ignoriert", "Un bloc sans lettres a été ignoré", "Un blocco senza lettere è stato ignorato", "文字のないブロックを無視しました", "没有字母的程序段已忽略", "Um bloco sem letras foi ignorado"]),
    ("d-code-invalid", ["The D code is not an operation or an aperture", "El código D no es una operación ni una apertura", "Der D-Code ist weder Operation noch Blende", "Le code D n'est ni une opération ni une ouverture", "Il codice D non è un'operazione né un'apertura", "D コードが操作でもアパーチャでもありません", "D 代码既不是操作也不是光圈", "O código D não é uma operação nem uma abertura"]),
    ("implied-operation", ["The operation is implied by the last one (deprecated)", "La operación se deduce de la anterior (obsoleto)", "Die Operation ergibt sich aus der letzten (veraltet)", "L'opération est déduite de la précédente (obsolète)", "L'operazione è dedotta dalla precedente (obsoleto)", "操作は直前のものから推定しました（非推奨）", "操作沿用上一个（已弃用）", "A operação deduz-se da anterior (obsoleto)"]),
    ("operation-missing", ["Coordinates without an operation to repeat", "Coordenadas sin operación que repetir", "Koordinaten ohne zu wiederholende Operation", "Coordonnées sans opération à répéter", "Coordinate senza operazione da ripetere", "繰り返す操作のない座標です", "坐标没有可沿用的操作", "Coordenadas sem operação a repetir"]),
    ("coordinate-missing", ["An operation uses a coordinate that was never given", "Una operación usa una coordenada que nunca se dio", "Eine Operation nutzt eine nie angegebene Koordinate", "Une opération utilise une coordonnée jamais donnée", "Un'operazione usa una coordinata mai data", "未指定の座標を使う操作です", "操作使用了从未给出的坐标", "Uma operação usa uma coordenada que nunca foi dada"]),
    ("start-point-missing", ["A draw starts before any move gave a start point", "Se dibuja antes de que un movimiento dé el punto de partida", "Es wird gezeichnet, bevor ein Startpunkt gesetzt ist", "On trace avant qu'un déplacement donne le point de départ", "Si disegna prima che un movimento dia il punto di partenza", "開始点が決まる前に描画しています", "在给出起点之前就开始绘制", "Desenha-se antes de um movimento dar o ponto de partida"]),
    ("header-missing", ["The file has no header", "El fichero no tiene cabecera", "Die Datei hat keinen Kopf", "Le fichier n'a pas d'en-tête", "Il file non ha intestazione", "ファイルにヘッダーがありません", "文件没有文件头", "O ficheiro não tem cabeçalho"]),
    ("format-comment-invalid", ["The format comment is invalid", "El comentario de formato no es válido", "Der Formatkommentar ist ungültig", "Le commentaire de format est invalide", "Il commento di formato non è valido", "フォーマットコメントが無効です", "格式注释无效", "O comentário de formato não é válido"]),
    ("defaults-invalid", ["The default format is invalid", "El formato por defecto no es válido", "Das Standardformat ist ungültig", "Le format par défaut est invalide", "Il formato predefinito non è valido", "既定のフォーマットが無効です", "默认格式无效", "O formato por omissão não é válido"]),
    ("option-unknown", ["Unknown option", "Opción desconocida", "Unbekannte Option", "Option inconnue", "Opzione sconosciuta", "未知のオプション", "未知的选项", "Opção desconhecida"]),
    ("tool-number-missing", ["The tool has no number", "La herramienta no tiene número", "Das Werkzeug hat keine Nummer", "L'outil n'a pas de numéro", "L'utensile non ha numero", "工具に番号がありません", "刀具没有编号", "A ferramenta não tem número"]),
    ("tool-diameter-invalid", ["The tool diameter is invalid", "El diámetro de la herramienta no es válido", "Der Werkzeugdurchmesser ist ungültig", "Le diamètre de l'outil est invalide", "Il diametro dell'utensile non è valido", "工具径が無効です", "刀具直径无效", "O diâmetro da ferramenta não é válido"]),
    ("tool-diameter-missing", ["The tool has no diameter", "La herramienta no tiene diámetro", "Das Werkzeug hat keinen Durchmesser", "L'outil n'a pas de diamètre", "L'utensile non ha diametro", "工具に径がありません", "刀具没有直径", "A ferramenta não tem diâmetro"]),
    ("tool-parameter-unknown", ["Unknown tool parameter", "Parámetro de herramienta desconocido", "Unbekannter Werkzeugparameter", "Paramètre d'outil inconnu", "Parametro di utensile sconosciuto", "未知の工具パラメータ", "未知的刀具参数", "Parâmetro de ferramenta desconhecido"]),
    ("tool-redefined", ["A tool is defined twice", "Una herramienta se define dos veces", "Ein Werkzeug ist doppelt definiert", "Un outil est défini deux fois", "Un utensile è definito due volte", "工具が二重に定義されています", "刀具被定义了两次", "Uma ferramenta é definida duas vezes"]),
    ("tool-undefined", ["The tool is not defined", "La herramienta no está definida", "Das Werkzeug ist nicht definiert", "L'outil n'est pas défini", "L'utensile non è definito", "工具が定義されていません", "刀具未定义", "A ferramenta não está definida"]),
    ("tool-selection-invalid", ["The tool selection is invalid", "La selección de herramienta no es válida", "Die Werkzeugwahl ist ungültig", "La sélection d'outil est invalide", "La selezione dell'utensile non è valida", "工具の選択が無効です", "刀具选择无效", "A seleção de ferramenta não é válida"]),
    ("tool-not-selected", ["A hole is drilled before a tool is selected", "Se taladra antes de elegir una herramienta", "Es wird gebohrt, bevor ein Werkzeug gewählt ist", "On perce avant de choisir un outil", "Si fora prima di scegliere un utensile", "工具を選ぶ前に穴をあけています", "在选择刀具之前就钻孔", "Fura-se antes de escolher uma ferramenta"]),
    ("arc-unsupported", ["This kind of arc is not supported", "Este tipo de arco no se admite", "Diese Art von Bogen wird nicht unterstützt", "Ce type d'arc n'est pas pris en charge", "Questo tipo di arco non è supportato", "この種類の円弧はサポートされていません", "不支持这种圆弧", "Este tipo de arco não é suportado"]),
    ("line-too-long", ["A line is too long and was cut", "Una línea es demasiado larga y se cortó", "Eine Zeile ist zu lang und wurde gekürzt", "Une ligne est trop longue et a été coupée", "Una riga è troppo lunga ed è stata tagliata", "行が長すぎるため切り詰めました", "行太长，已截断", "Uma linha é demasiado longa e foi cortada"]),
    ("block-delete-read", ["A block marked for deletion was read", "Se leyó un bloque marcado para borrar", "Ein zum Löschen markierter Block wurde gelesen", "Un bloc marqué à supprimer a été lu", "È stato letto un blocco marcato per l'eliminazione", "削除指定のブロックを読み込みました", "读取了标记为跳过的程序段", "Foi lido um bloco marcado para eliminar"]),
    ("system-command-ignored", ["A system command was ignored", "Se ignoró una orden del sistema", "Ein Systembefehl wurde ignoriert", "Une commande système a été ignorée", "Un comando di sistema è stato ignorato", "システムコマンドを無視しました", "系统命令已忽略", "Um comando de sistema foi ignorado"]),
    ("checksum-mismatch", ["The checksum does not match", "La suma de comprobación no coincide", "Die Prüfsumme stimmt nicht", "La somme de contrôle ne correspond pas", "Il checksum non corrisponde", "チェックサムが一致しません", "校验和不匹配", "A soma de verificação não coincide"]),
    ("checksum-invalid", ["The checksum is not a number", "La suma de comprobación no es un número", "Die Prüfsumme ist keine Zahl", "La somme de contrôle n'est pas un nombre", "Il checksum non è un numero", "チェックサムが数値ではありません", "校验和不是数字", "A soma de verificação não é um número"]),
    ("comment-unclosed", ["A comment is never closed", "Un comentario no se cierra nunca", "Ein Kommentar wird nie geschlossen", "Un commentaire n'est jamais fermé", "Un commento non viene mai chiuso", "コメントが閉じられていません", "注释从未关闭", "Um comentário nunca é fechado"]),
    ("word-number-invalid", ["A word has no valid number", "Una palabra no tiene un número válido", "Ein Wort hat keine gültige Zahl", "Un mot n'a pas de nombre valide", "Una parola non ha un numero valido", "ワードに有効な数値がありません", "字缺少有效数字", "Uma palavra não tem um número válido"]),
    ("character-ignored", ["A character was ignored", "Se ignoró un carácter", "Ein Zeichen wurde ignoriert", "Un caractère a été ignoré", "Un carattere è stato ignorato", "文字を無視しました", "一个字符已忽略", "Um carácter foi ignorado"]),
    ("plane-unsupported", ["An arc in another plane is not supported", "Un arco en otro plano no se admite", "Ein Bogen in einer anderen Ebene wird nicht unterstützt", "Un arc dans un autre plan n'est pas pris en charge", "Un arco su un altro piano non è supportato", "他の平面の円弧はサポートされていません", "不支持其他平面中的圆弧", "Um arco noutro plano não é suportado"]),
    ("cutter-compensation-unsupported", ["Cutter compensation is not supported", "La compensación de herramienta no se admite", "Die Fräserkorrektur wird nicht unterstützt", "La compensation d'outil n'est pas prise en charge", "La compensazione dell'utensile non è supportata", "工具径補正はサポートされていません", "不支持刀具补偿", "A compensação de ferramenta não é suportada"]),
    ("canned-cycle-unsupported", ["A canned cycle is not supported", "Un ciclo fijo no se admite", "Ein Bohrzyklus wird nicht unterstützt", "Un cycle fixe n'est pas pris en charge", "Un ciclo fisso non è supportato", "固定サイクルはサポートされていません", "不支持固定循环", "Um ciclo fixo não é suportado"]),
    ("negative-feed", ["The feed rate is negative", "El avance es negativo", "Der Vorschub ist negativ", "L'avance est négative", "L'avanzamento è negativo", "送り速度が負です", "进给速度为负", "O avanço é negativo"]),
    ("negative-spindle", ["The spindle speed is negative", "La velocidad del husillo es negativa", "Die Spindeldrehzahl ist negativ", "La vitesse de broche est négative", "La velocità del mandrino è negativa", "主軸速度が負です", "主轴转速为负", "A velocidade do fuso é negativa"]),
    ("tool-number-invalid", ["The tool number is invalid", "El número de herramienta no es válido", "Die Werkzeugnummer ist ungültig", "Le numéro d'outil est invalide", "Il numero dell'utensile non è valido", "工具番号が無効です", "刀具编号无效", "O número da ferramenta não é válido"]),
    ("implied-rapid", ["A rapid move is implied", "Se supone un movimiento rápido", "Eine Eilgangbewegung wird angenommen", "Un déplacement rapide est supposé", "Si presume un movimento rapido", "早送りとみなしました", "按快速移动处理", "Assume-se um movimento rápido"]),
    ("arc-radius-unreachable", ["The arc radius cannot reach the end point", "El radio del arco no llega al punto final", "Der Bogenradius erreicht den Endpunkt nicht", "Le rayon de l'arc n'atteint pas le point final", "Il raggio dell'arco non raggiunge il punto finale", "円弧の半径が終点に届きません", "圆弧半径够不到终点", "O raio do arco não chega ao ponto final"]),
    ("arc-centre-missing", ["The arc has no centre", "El arco no tiene centro", "Der Bogen hat keinen Mittelpunkt", "L'arc n'a pas de centre", "L'arco non ha centro", "円弧の中心がありません", "圆弧没有圆心", "O arco não tem centro"]),
    ("arc-radius-mismatch", ["The two radii of the arc differ", "Los dos radios del arco difieren", "Die beiden Radien des Bogens weichen ab", "Les deux rayons de l'arc diffèrent", "I due raggi dell'arco differiscono", "円弧の二つの半径が一致しません", "圆弧的两个半径不一致", "Os dois raios do arco diferem"]),
    ("program-too-long", ["The program is too long and was cut", "El programa es demasiado largo y se cortó", "Das Programm ist zu lang und wurde gekürzt", "Le programme est trop long et a été coupé", "Il programma è troppo lungo ed è stato tagliato", "プログラムが長すぎるため切り詰めました", "程序太长，已截断", "O programa é demasiado longo e foi cortado"]),
    ("units-suspect", ["The units look wrong for the moves", "Las unidades parecen erróneas para los movimientos", "Die Einheiten passen offenbar nicht zu den Bewegungen", "Les unités semblent erronées pour les déplacements", "Le unità sembrano errate per i movimenti", "移動量に対して単位が不自然です", "单位与移动量不符", "As unidades parecem erradas para os movimentos"]),
    ("units-forced", ["The units were set by the caller", "Las unidades las fijó quien llama", "Die Einheiten wurden vom Aufrufer gesetzt", "Les unités ont été imposées par l'appelant", "Le unità sono state imposte dal chiamante", "単位は呼び出し側が指定しました", "单位由调用方指定", "As unidades foram definidas por quem chama"]),
    ("feed-missing", ["A cutting move has no feed rate", "Un movimiento de corte no tiene avance", "Eine Schnittbewegung hat keinen Vorschub", "Un déplacement de coupe n'a pas d'avance", "Un movimento di taglio non ha avanzamento", "切削移動に送り速度がありません", "切削移动没有进给速度", "Um movimento de corte não tem avanço"]),
    ("reference-position-unset", ["The reference position is not set", "La posición de referencia no está fijada", "Die Referenzposition ist nicht gesetzt", "La position de référence n'est pas définie", "La posizione di riferimento non è impostata", "基準位置が設定されていません", "未设置参考位置", "A posição de referência não está definida"]),
    ("g53-motion-unsupported", ["A move in machine coordinates is not supported", "Un movimiento en coordenadas de máquina no se admite", "Eine Bewegung in Maschinenkoordinaten wird nicht unterstützt", "Un déplacement en coordonnées machine n'est pas pris en charge", "Un movimento in coordinate macchina non è supportato", "機械座標での移動はサポートされていません", "不支持机床坐标移动", "Um movimento em coordenadas de máquina não é suportado"]),
    ("g92-axes-missing", ["G92 names no axis", "G92 no nombra ningún eje", "G92 nennt keine Achse", "G92 ne nomme aucun axe", "G92 non nomina alcun asse", "G92 に軸の指定がありません", "G92 没有指定轴", "G92 não indica nenhum eixo"]),
    ("offset-out-of-range", ["The offset is out of range", "El desplazamiento está fuera de rango", "Der Versatz liegt außerhalb des Bereichs", "Le décalage est hors limites", "Lo scostamento è fuori intervallo", "オフセットが範囲外です", "偏移超出范围", "O desvio está fora do intervalo"]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codes, Severity};

    #[test]
    fn every_code_has_its_eight_headlines() {
        for code in codes::ALL {
            for language in Language::ALL {
                let text = headline(code, language);
                assert!(
                    text.is_some_and(|t| !t.trim().is_empty()),
                    "{code} has no {} headline",
                    language.tag()
                );
            }
        }
        assert_eq!(
            TABLE.len(),
            codes::ALL.len(),
            "a row of the table has no code"
        );
    }

    #[test]
    fn the_table_follows_the_order_of_the_codes_and_has_no_repeats() {
        for (row, code) in TABLE.iter().zip(codes::ALL) {
            assert_eq!(row.0, *code);
        }
    }

    #[test]
    fn portuguese_is_the_one_of_portugal_and_not_spanish_or_brazilian() {
        for (code, texts) in TABLE {
            assert_ne!(texts[1], texts[7], "{code}: pt equals es");
        }
        let all_pt: String = TABLE
            .iter()
            .map(|(_, t)| t[7])
            .collect::<Vec<_>>()
            .join(" ");
        for brazilian in ["arquivo", "tela ", "usuário", "salvar", "planilha"] {
            assert!(!all_pt.contains(brazilian), "{brazilian}");
        }
        assert!(all_pt.contains("ficheiro"));
    }

    #[test]
    fn no_headline_of_another_language_is_the_english_one() {
        for (code, texts) in TABLE {
            for (i, text) in texts.iter().enumerate().skip(1) {
                if matches!(*code, "deprecated-command" | "unknown-command") && i >= 3 {
                    continue;
                }
                assert_ne!(*text, texts[0], "{code} language #{i}");
            }
        }
    }

    #[test]
    fn a_tag_gives_its_language() {
        assert_eq!(Language::from_tag("pt-PT"), Some(Language::Pt));
        assert_eq!(Language::from_tag("ZH_cn"), Some(Language::Zh));
        assert_eq!(Language::from_tag("ja"), Some(Language::Ja));
        assert_eq!(Language::from_tag("xx"), None);
        assert_eq!(Language::from_tag(""), None);
        assert_eq!(Language::from_tag("é"), None);
        for language in Language::ALL {
            assert_eq!(Language::from_tag(language.tag()), Some(language));
        }
    }

    #[test]
    fn a_notice_is_written_in_the_language_asked() {
        let d = Diagnostic::new(7, Severity::Error, codes::REGION_UNCLOSED, "x");
        assert_eq!(
            localize(&d, Language::Pt),
            "Uma região nunca é fechada [region-unclosed] (linha 7)"
        );
        assert_eq!(
            localize(&d, Language::En),
            "A region is never closed [region-unclosed] (line 7)"
        );
        assert!(localize(&d, Language::Ja).contains("リージョン"));
        assert!(headline("no-such-code", Language::En).is_none());
    }
}
