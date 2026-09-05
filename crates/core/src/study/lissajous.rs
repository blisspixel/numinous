//! The bilingual pilot: one scientific structure with paired human prose.
//!
//! The source drafts and independent bilingual review were prepared on
//! 2026-09-05. Equations, inline notation and source identities are declared
//! once. The review's time-coordinate, independence, occupation-limit and
//! angular-frequency clarifications apply to both languages below.

use super::{
    StudyBlock, StudyDepth, StudyInline, StudyLocaleResolution, StudyPart, StudySource,
    StudyTranslationStatus,
};

enum Inline {
    Prose(&'static str, &'static str),
    Math(&'static str),
}

const fn t(english: &'static str, japanese: &'static str) -> Inline {
    Inline::Prose(english, japanese)
}

const fn m(notation: &'static str) -> Inline {
    Inline::Math(notation)
}

fn language(japanese: bool, english: &'static str, translated: &'static str) -> &'static str {
    if japanese { translated } else { english }
}

fn paragraph(japanese: bool, runs: &[Inline]) -> StudyPart {
    StudyPart::Paragraph(
        runs.iter()
            .map(|run| match run {
                Inline::Prose(english, translated) => {
                    StudyInline::Text(language(japanese, english, translated))
                }
                Inline::Math(notation) => StudyInline::Math(notation),
            })
            .collect(),
    )
}

fn block(
    locale: &StudyLocaleResolution,
    id: &'static str,
    english_title: &'static str,
    japanese_title: &'static str,
    depth: StudyDepth,
    parts: Vec<StudyPart>,
) -> StudyBlock {
    let japanese = locale.resolved == "ja";
    StudyBlock {
        id: id.to_string(),
        title: language(japanese, english_title, japanese_title),
        depth,
        locale: locale.clone(),
        translation: if japanese {
            StudyTranslationStatus::ReviewedDraft
        } else {
            StudyTranslationStatus::Original
        },
        parts,
    }
}

pub(super) fn blocks(locale: &StudyLocaleResolution) -> Vec<StudyBlock> {
    let japanese = locale.resolved == "ja";
    let p = |runs: &[Inline]| paragraph(japanese, runs);
    let r = |source, english, translated| StudyPart::Reference {
        source,
        description: language(japanese, english, translated),
    };
    vec![
        block(
            locale,
            "lissajous.try",
            "Tune a relationship",
            "二つの振動の関係を調律する",
            StudyDepth::Explanation,
            vec![
                p(&[
                    t(
                        "Move the tuning position sideways, then vertically. In the App, drag to do this. Which number in the ",
                        "調律する位置を横に動かして、次は縦に。Appではドラッグして操作します。",
                    ),
                    m("X:Y"),
                    t(
                        " readout changes each time? Find ",
                        "の表示では、それぞれどちらの数が変わるでしょうか。",
                    ),
                    m("2:3"),
                    t(", then ", "を見つけたら、今度は"),
                    m("4:6"),
                    t(
                        ". With room audio active, compare their sound. What did doubling both numbers preserve?",
                        "へ。ルームの音が有効なら、聴き比べてみましょう。二つの数をどちらも二倍にすると、何が変わらずに残るでしょうか。",
                    ),
                ]),
                p(&[
                    t("Try ", ""),
                    m("1:1"),
                    t(
                        " and let the shape change without touching it. In the App, use the wheel to see how the shape changes. Can you find a circle, then a thin line, while the frequency readout stays put?",
                        "にして、触らずに形の変化を眺めてみましょう。Appではホイールを回して、形がどう変わるか見てみましょう。振動の速さの表示を変えずに、円を見つけ、次に細い線を見つけられるでしょうか。",
                    ),
                ]),
                p(&[t(
                    "Keep playing, or open any explanation or proof whenever you like. Every section is optional; you can start at any depth.",
                    "このまま自由に遊んでも、気になった解説や証明へ進んでもかまいません。どの項目も任意で、詳しい数学から読み始めることもできます。",
                )]),
            ],
        ),
        block(
            locale,
            "lissajous.intuition",
            "Two clocks, one drawing",
            "二つの時計が、一つの図形を描く",
            StudyDepth::Explanation,
            vec![
                p(&[t(
                    "One oscillator moves the point horizontally; another moves it vertically. Their frequencies set how many cycles each makes. Their phases set where each starts within its cycle. A frequency ratio can stay fixed while the shape changes.",
                    "一方の振動子が点を横に動かし、もう一方が縦に動かします。振動数は、それぞれが何回振動するかを決めます。位相は、各周期のどの位置から始めるかを決めます。振動数の比が同じでも、形は変わります。",
                )]),
                p(&[t(
                    "If both clocks complete whole numbers of cycles together, the entire motion repeats. A crossing in the drawing tells you less: the point may be passing the same place in a different direction. Remembering direction lets you predict which way it will leave.",
                    "二つの時計が同時に、それぞれ整数回の周期を終えると、運動全体が繰り返されます。図の交点だけでは、そこまではわかりません。同じ場所を違う向きに通過しているかもしれないからです。向きも覚えておけば、その点が次にどちらへ進むかを予測できます。",
                )]),
                p(&[
                    t(
                        "For matching initial phases, ",
                        "二つの設定で初期位相を同じにすると、",
                    ),
                    m("2:3"),
                    t(" and ", "と"),
                    m("4:6"),
                    t(
                        " have the same ideal trace. The second traverses it twice as fast in oscillator time, and its two audio frequencies are an octave higher. Sampling, accumulated strokes, and changing gallery phase can make their displayed pixels differ.",
                        "は、理想的には同じ軌跡を描きます。後者は振動の時間パラメータに沿って二倍の速さで軌跡をたどり、音の二つの周波数は一オクターブ高くなります。サンプリング、線の重なり、ギャラリー位相の変化によって、実際に表示される画素は異なることがあります。",
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "lissajous.equations",
            "The equations and the least period",
            "方程式と最小周期",
            StudyDepth::Mathematics,
            vec![
                p(&[t(
                    "Freeze the controls conceptually. In the ideal model,",
                    "ここでは、操作で変わる設定を固定したと考えます。理想モデルでは、",
                )]),
                StudyPart::Equation(
                    "x(theta) = cos(a*theta + alpha)\ny(theta) = sin(b*theta + beta),        a > 0, b > 0\nx'' = -a^2*x,                        y'' = -b^2*y",
                ),
                p(&[
                    t("Primes mean differentiation with respect to ", "プライムは"),
                    m("theta"),
                    t(
                        ". This oscillator parameter is separate from the gallery control ",
                        "による微分を表します。この振動のパラメータは、ギャラリーを操作する",
                    ),
                    m("t"),
                    t(
                        ". Unit amplitude means ",
                        "とは別のものです。振幅が一なので、",
                    ),
                    m("x^2 + (x'/a)^2 = y^2 + (y'/b)^2 = 1"),
                    t(".", "が成り立ちます。"),
                ]),
                p(&[
                    t(
                        "Here a and b are angular frequencies relative to theta: they measure phase advance per unit of theta. The corresponding numbers of cycles per unit of theta are ",
                        "ここでaとbは、thetaに対する角振動数、つまり位相が進む速さです。thetaの一単位あたりの振動回数は、それぞれ",
                    ),
                    m("a/(2*pi)"),
                    t(" and ", "と"),
                    m("b/(2*pi)"),
                    t(
                        ". The audio mapping separately uses ",
                        "です。音声では、別の対応づけによって",
                    ),
                    m("110*a"),
                    t(" and ", "Hzと"),
                    m("110*b"),
                    t(" Hz.", "Hzを使います。"),
                ]),
                p(&[t(
                    "A positive full period requires",
                    "運動全体が繰り返される正の周期には、次の条件が必要です。",
                )]),
                StudyPart::Equation("a*T = 2*pi*m,     b*T = 2*pi*n,       m,n positive integers."),
                p(&[
                    t(
                        "Necessity follows because position and normalized velocity together determine each phase modulo ",
                        "位置と正規化した速度がそろえば、各位相が",
                    ),
                    m("2*pi"),
                    t(
                        ". Sufficiency follows by substituting these increments. Thus a period exists exactly when ",
                        "を法として定まります。これが必要性の理由です。十分性は、この位相の増分を代入すればわかります。したがって、周期が存在するのは",
                    ),
                    m("b/a"),
                    t(
                        " is rational.",
                        "が有理数である場合、かつその場合に限られます。",
                    ),
                ]),
                p(&[
                    t("Write ", ""),
                    m("a = m*Omega"),
                    t(", ", "、"),
                    m("b = n*Omega"),
                    t(" with coprime positive integers ", "と、互いに素な正の整数"),
                    m("m,n"),
                    t(". If ", "を使って書きます。"),
                    m("r = Omega*T/(2*pi)"),
                    t(", then both ", "とおくと、"),
                    m("m*r"),
                    t(" and ", "と"),
                    m("n*r"),
                    t(
                        " must be integers. Bezout's identity gives integers ",
                        "はどちらも整数でなければなりません。ベズーの等式により、整数",
                    ),
                    m("u,v"),
                    t(" with ", "で"),
                    m("u*m + v*n = 1"),
                    t(", so ", "を満たすものが存在するので、"),
                    m("r"),
                    t(
                        " is an integer too. Consequently the least positive period is ",
                        "も整数です。したがって、最小の正の周期は",
                    ),
                    m("T0 = 2*pi/Omega"),
                    t(".", "です。"),
                ]),
                p(&[
                    t(
                        "For the room's integer position tunings, ",
                        "このルームで位置を操作して整数に調律した場合は、",
                    ),
                    m("Omega = gcd(a,b)"),
                    t(". Hence ", "です。そのため、"),
                    m("2:3"),
                    t(" has ", "では"),
                    m("T0 = 2*pi"),
                    t(", ", "、"),
                    m("4:6"),
                    t(" has ", "では"),
                    m("T0 = pi"),
                    t(", and ", "、"),
                    m("6:8"),
                    t(" has ", "でも"),
                    m("T0 = pi"),
                    t(
                        ". Reducing a ratio tells you the winding counts; retaining its common factor tells you the speed.",
                        "となります。比を約分すると巻き数がわかり、共通因子を残しておくと速さがわかります。",
                    ),
                ]),
                r(
                    &MIT,
                    "The model and its periodic/quasiperiodic distinction.",
                    "このモデルと周期的・準周期的な運動の違い。",
                ),
            ],
        ),
        block(
            locale,
            "lissajous.state",
            "Home is more than a place",
            "戻るのは、場所だけではない",
            StudyDepth::Mathematics,
            vec![
                p(&[
                    t(
                        "Use the room's untouched sweep at gallery ",
                        "位置を操作して調律していない自動変化を使い、ギャラリーを",
                    ),
                    m("t = 0.5"),
                    t(", variation ", "、バリエーションを"),
                    m("0"),
                    t(":", "にします。"),
                ]),
                StudyPart::Equation(
                    "x(theta) = cos(3*theta)\ny(theta) = sin(3.5*theta)\n\ntheta = 0:       (x,y) = (1,0),       (x',y') = (0, 3.5)\ntheta = 2*pi:    (x,y) = (1,0),       (x',y') = (0,-3.5)",
                ),
                p(&[
                    t(
                        "The position returns at the end of the displayed oscillator window, but its vertical velocity reverses. The full period is ",
                        "表示している振動の区間の終わりで位置は元に戻りますが、縦方向の速度は反転します。運動全体の周期は",
                    ),
                    m("4*pi"),
                    t(
                        ", because the angular frequencies are ",
                        "です。二つの角振動数が",
                    ),
                    m("6*(1/2)"),
                    t(" and ", "と"),
                    m("7*(1/2)"),
                    t(
                        ". An endpoint match is not a period test. Even a periodic trajectory may project to a segment that it retraces, rather than a simple loop.",
                        "だからです。始点と終点が一致するだけでは、周期の確認にはなりません。周期的な軌道であっても、その投影が単純な輪ではなく、同じ道を往復する曲線になることがあります。",
                    ),
                ]),
                p(&[
                    t(
                        "Carry the distinction into Studio's existing Same place, another direction capsule: ",
                        "この違いを、StudioにあるSame place, another direction（同じ場所、違う向き）のカプセルでも確かめてみましょう。式は",
                    ),
                    m("x(t) = cos(4*pi*t)"),
                    t(", ", "、"),
                    m("y(t) = sin(6*pi*t)"),
                    t(". At ", "です。"),
                    m("t = 0"),
                    t(" and ", "と"),
                    m("t = 0.5"),
                    t(" the point is ", "で点の位置はどちらも"),
                    m("(1,0)"),
                    t(
                        ", but its vertical velocities are ",
                        "ですが、縦方向の速度はそれぞれ",
                    ),
                    m("6*pi"),
                    t(" and ", "と"),
                    m("-6*pi"),
                    t(". Its full period is ", "です。運動全体の周期は"),
                    m("1"),
                    t(".", "です。"),
                ]),
                p(&[
                    t(
                        "This Studio capsule uses the room's zero-phase ",
                        "このStudioカプセルは、初期位相をどちらも零とした、ルームの",
                    ),
                    m("2:3"),
                    t(" ideal model, ", "の理想モデル、"),
                    m("x(theta) = cos(2*theta)"),
                    t(", ", "、"),
                    m("y(theta) = sin(3*theta)"),
                    t(
                        ", with the time-coordinate change ",
                        "に対応します。時間パラメータの対応は",
                    ),
                    m("theta = 2*pi*t"),
                    t(".", "です。"),
                ]),
                p(&[t(
                    "Follow the playable capsules in docs/experiments/returning-home.md.",
                    "遊べるカプセルはdocs/experiments/returning-home.mdからたどれます。",
                )]),
            ],
        ),
        block(
            locale,
            "lissajous.phase",
            "A phase becomes geometry",
            "位相が形になる",
            StudyDepth::Mathematics,
            vec![
                p(&[
                    t(
                        "At equal angular frequency, let ",
                        "二つの角振動数が等しいとき、",
                    ),
                    m("u = a*theta + alpha"),
                    t(" and ", "、"),
                    m("delta = beta - alpha"),
                    t(". Then ", "とおきます。すると"),
                    m("x = cos(u)"),
                    t(" and", "であり、"),
                ]),
                StudyPart::Equation(
                    "y = sin(u + delta) = sin(u)*cos(delta) + x*sin(delta)\n(y - x*sin(delta))^2 = (1 - x^2)*cos(delta)^2\nx^2 + y^2 - 2*x*y*sin(delta) = cos(delta)^2.",
                ),
                p(&[
                    t("At ", ""),
                    m("delta = 0"),
                    t(" or ", "または"),
                    m("pi"),
                    t(" this is the unit circle. At ", "では単位円です。"),
                    m("delta = pi/2"),
                    t(" it is the segment ", "では線分"),
                    m("y = x"),
                    t(", with ", "であり、その範囲は"),
                    m("-1 <= x <= 1"),
                    t("; at ", "です。"),
                    m("3*pi/2"),
                    t(" it is ", "では"),
                    m("y = -x"),
                    t(
                        ". Intermediate phases give ellipses. The linear map from ",
                        "になります。その間の位相では楕円になります。",
                    ),
                    m("(cos(u),sin(u))"),
                    t(" to ", "から"),
                    m("(x,y)"),
                    t(" has determinant ", "への線形写像の行列式は"),
                    m("cos(delta)"),
                    t(", so the ellipse's area is ", "なので、楕円の面積は"),
                    m("pi*abs(cos(delta))"),
                    t(".", "です。"),
                ]),
                p(&[
                    t(
                        "No pitch changed. The phase changed how two motions meet. A circle alone also does not tell you its direction of traversal: ",
                        "音の高さは変わっていません。位相が、二つの運動の組み合わさり方を変えたのです。円を見ただけでは、どちら向きにたどるかもわかりません。",
                    ),
                    m("delta = 0"),
                    t(" and ", "と"),
                    m("pi"),
                    t(
                        " trace the same circle with opposite orientations in mathematical coordinates.",
                        "は同じ円を描きますが、数学的な座標では逆向きにたどります。",
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "lissajous.torus",
            "The drawing is a shadow of a torus",
            "図形はトーラスの影",
            StudyDepth::Mathematics,
            vec![
                p(&[t(
                    "Each oscillator phase is a point on a circle. The full fixed-amplitude state is therefore the two-dimensional torus",
                    "各振動子の位相は、円周上の一点です。したがって、振幅を固定したときの状態全体は、二次元トーラスで表されます。",
                )]),
                StudyPart::Equation(
                    "(u,v) = (a*theta + alpha, b*theta + beta) modulo 2*pi\nS = (x, x'/a, y, y'/b) = (cos(u), -sin(u), sin(v), cos(v)).",
                ),
                p(&[t(
                    "The drawing keeps only two of those four state coordinates. This projection folds distinct states onto the same position.",
                    "図が残すのは、この四つの状態座標のうち二つだけです。この投影によって、異なる状態が同じ位置に重なります。",
                )]),
                p(&[
                    t("For ", ""),
                    m("a = m*Omega"),
                    t(", ", "、"),
                    m("b = n*Omega"),
                    t(" with coprime ", "と、互いに素な"),
                    m("m,n"),
                    t(", ", "を使って書ける場合、"),
                    m("n*u - m*v = n*alpha - m*beta"),
                    t(" modulo ", "は、"),
                    m("2*pi"),
                    t(
                        " is invariant: differentiating gives ",
                        "を法として不変です。微分すると",
                    ),
                    m("n*a - m*b = 0"),
                    t(
                        ". A periodic orbit winds ",
                        "となるからです。周期軌道は、一方の位相円を",
                    ),
                    m("m"),
                    t(" times around one phase circle and ", "回、他方を"),
                    m("n"),
                    t(
                        " around the other. Its phase invariant helps distinguish orbits with the same winding ratio. It is more informative than subtracting phases when the angular frequencies differ.",
                        "回まわります。この位相の不変量は、巻き数の比が同じ軌道どうしを区別する手がかりになります。角振動数が異なるときには、単に位相の差を取るより多くの情報を持ちます。",
                    ),
                ]),
                p(&[
                    t("For irrational ", ""),
                    m("b/a"),
                    t(
                        ", take a torus Fourier character ",
                        "が無理数の場合、トーラス上のフーリエ指標",
                    ),
                    m("exp(i*(k*u+l*v))"),
                    t(" with integers ", "を、整数"),
                    m("(k,l) != (0,0)"),
                    t(". Its angular frequency ", "に対して考えます。その角振動数"),
                    m("lambda = k*a+l*b"),
                    t(" is nonzero, and", "は零ではなく、"),
                ]),
                StudyPart::Equation(
                    "(1/L) integral_0^L exp(i*(k*u(theta)+l*v(theta))) dtheta\n  = exp(i*(k*alpha+l*beta)) * (exp(i*lambda*L)-1)/(i*lambda*L).",
                ),
                p(&[
                    t("The magnitude is at most ", "絶対値は高々"),
                    m("2/(L*abs(lambda))"),
                    t(
                        ", which tends to zero. Finite sums and uniform approximation by trigonometric polynomials extend this to every continuous observable: its long-time average equals its uniform torus average, for every initial phase. This proves equidistribution and therefore density. Small ",
                        "で、零に近づきます。有限和を取り、さらに三角多項式による一様近似を使うと、この結果はすべての連続な観測量に広がります。どの初期位相から始めても、長時間平均はトーラス上の一様な測度による平均と一致します。これで一様分布性が証明され、したがって稠密性もわかります。",
                    ),
                    m("abs(lambda)"),
                    t(
                        " also explains why an apparently resonant finite observation can persist a long time.",
                        "が小さいことは、有限の観測では共鳴しているように見える状態が長く続きうる理由にもなります。",
                    ),
                ]),
                r(
                    &ERGODIC,
                    "The Fourier argument and uniform approximation underlying this continuous-flow calculation.",
                    "この連続時間の流れの計算に対応する、フーリエ解析による議論と一様近似。",
                ),
            ],
        ),
        block(
            locale,
            "lissajous.measure",
            "Dense does not mean uniform in the picture",
            "稠密でも、図の中で一様とは限らない",
            StudyDepth::Mathematics,
            vec![
                p(&[
                    t(
                        "For the irrational ideal flow, uniform torus phase pushes forward under ",
                        "角振動数の比が無理数である理想的な流れでは、トーラス上の一様な位相分布を",
                    ),
                    m("x = cos(u)"),
                    t(" to the density ", "で押し出すと、密度は"),
                    m("1/(pi*sqrt(1-x^2))"),
                    t(
                        ". There are two phase preimages for an interior x value, each contributing ",
                        "になります。xの値が区間の内部にあるとき、それに対応する位相は二つあります。それぞれが",
                    ),
                    m("1/(2*pi*abs(sin(u)))"),
                    t(
                        " by change of variables.",
                        "だけ寄与することが、変数変換でわかります。",
                    ),
                ]),
                p(&[
                    t(
                        "The y coordinate has the same marginal density. Under the normalized uniform torus measure ",
                        "y座標の周辺密度も同じです。正規化した一様なトーラス測度",
                    ),
                    m("dmu = du*dv/(2*pi)^2"),
                    t(
                        ", the phase coordinates u and v are independent. Since x depends only on u and y only on v, their joint density is the product of the two marginal densities. Thus the occupation density inside the square is",
                        "では、位相uとvは独立です。xはuだけに、yはvだけに依存するので、同時密度は二つの周辺密度の積になります。したがって、正方形の内部での滞在時間の割合を表す密度は、",
                    ),
                ]),
                StudyPart::Equation("rho(x,y) = 1/(pi^2*sqrt(1-x^2)*sqrt(1-y^2))."),
                p(&[
                    t(
                        "As observation time tends to infinity, the fraction of ideal oscillator time with ",
                        "理想軌道の観測時間を限りなく長くすると、",
                    ),
                    m("abs(x) < 1/2"),
                    t(" tends to ", "に滞在する時間の割合は"),
                    m("1/3"),
                    t(", and the fraction with both ", "に近づき、"),
                    m("abs(x) < 1/2"),
                    t(" and ", "と"),
                    m("abs(y) < 1/2"),
                    t(" tends to ", "を同時に満たす時間の割合は"),
                    m("1/9"),
                    t(
                        ". The central square has ",
                        "に近づきます。一方、中央の正方形の面積は、全体の",
                    ),
                    m("1/4"),
                    t(
                        " of the total geometric area. These long-time occupation fractions and area fractions differ: each coordinate moves slowly near its turning points. The boundary has zero measure, so the equidistribution result also gives these region-occupation limits.",
                        "です。長時間の滞在割合と面積の割合は異なります。各座標の動きが、折り返し点の近くで遅くなるためです。領域の境界の測度は零なので、一様分布性の結果を、この領域での滞在割合の極限にも適用できます。",
                    ),
                ]),
                p(&[
                    t(
                        "Nor does density imply mixing. For the zero-mean observable ",
                        "稠密であることから混合性が従うわけでもありません。平均零の観測量",
                    ),
                    m("f(u,v) = exp(i*u)"),
                    t(
                        ", its phase-space correlation after elapsed oscillator time ",
                        "について、振動の時間",
                    ),
                    m("h"),
                    t(" is ", "が経過した後の相空間上の相関は"),
                    m("integral f(u+a*h,v+b*h)*conj(f(u,v)) dmu = exp(i*a*h)"),
                    t(
                        ". Its magnitude remains one instead of decaying to zero. The system can be dense and recurrent while retaining this exact phase correlation. It is not a source of independent random samples.",
                        "です。その絶対値は零へ減衰せず、一のままです。この系は稠密で再帰的でありながら、この厳密な位相相関を保つことができます。独立な乱数標本を生むものではありません。",
                    ),
                ]),
                p(&[t(
                    "These predictions concern the ideal trajectory's occupation times. The App's short trace, overlapping raster strokes, and glow are not a calibrated density plot. The torus block supplies the uniform phase measure; the formulas here follow by change of variables and direct integration.",
                    "これらは理想軌道の滞在時間についての予測です。Appの短い軌跡、ラスタ上で重なる線、発光表現は、定量的に校正された密度図ではありません。トーラスの項目の結果が一様な位相測度を与え、ここでの式は変数変換と直接の積分から導かれます。",
                )]),
            ],
        ),
        block(
            locale,
            "lissajous.recurrence",
            "Almost home, with an error bound",
            "もう少しで元どおり、その誤差を測る",
            StudyDepth::Mathematics,
            vec![
                p(&[
                    t("Let ", ""),
                    m("r = b/a"),
                    t(" be irrational. At ", "を無理数とします。"),
                    m("Tq = 2*pi*q/a"),
                    t(
                        ", the x phase returns exactly. Choose an integer ",
                        "では、xの位相が厳密に元に戻ります。整数",
                    ),
                    m("p"),
                    t(" close to ", "を"),
                    m("q*r"),
                    t(
                        "; the remaining y-phase error is ",
                        "に近く選ぶと、残るyの位相誤差は",
                    ),
                    m("d = 2*pi*(q*r-p)"),
                    t(
                        ". For the normalized full state ",
                        "です。正規化した状態全体",
                    ),
                    m("S = (x, x'/a, y, y'/b)"),
                    t(",", "について、"),
                ]),
                StudyPart::Equation(
                    "sup_theta ||S(theta+Tq)-S(theta)||_2 = 2*abs(sin(d/2)) <= abs(d).",
                ),
                p(&[
                    t("For a positive integer Q, partition ", "正の整数Qを選び、"),
                    m("[0,1)"),
                    t(
                        " into Q equal intervals. Place the ",
                        "をQ個の等しい区間に分けます。",
                    ),
                    m("Q+1"),
                    t(" fractional parts of ", "個の数"),
                    m("0,r,2*r,...,Q*r"),
                    t(
                        " in them. Two share an interval. Subtracting their indices gives ",
                        "の小数部分を振り分けます。少なくとも二つが同じ区間に入ります。それらの添字の差を取ると、",
                    ),
                    m("1 <= q <= Q"),
                    t(" and an integer p with ", "と、"),
                    m("abs(q*r-p) < 1/Q"),
                    t(
                        ". The entire normalized state thus repeats within ",
                        "を満たす整数pが得られます。したがって、正規化した状態全体は、",
                    ),
                    m("2*pi/Q"),
                    t(
                        ", uniformly in starting time. For irrational r these q values must become unbounded as the error tolerance shrinks. This is an approximate repeat of the motion, not just a lucky position crossing.",
                        "未満の誤差で、どの開始時刻から見ても一様に再現されます。rが無理数なら、許容誤差を小さくしていくにつれて、これらのqの値は有界ではいられません。単に同じ位置を偶然通るだけでなく、運動が近似的に再現されるのです。",
                    ),
                ]),
                r(
                    &DIRICHLET,
                    "Dirichlet approximation applied to recurrence.",
                    "ディリクレ近似を再帰に適用する議論。",
                ),
                p(&[
                    t(
                        "For the Almost home Studio capsule, ",
                        "StudioのAlmost home（もう少しで元どおり）カプセルでは、",
                    ),
                    m("r = sqrt(2)"),
                    t(". Truncating the continued fraction ", "です。連分数"),
                    m("[1;2,2,2,...]"),
                    t(
                        " at successive depths gives its convergents, including ",
                        "を途中で打ち切って得られる近似分数には、",
                    ),
                    m("17/12"),
                    t(", ", "、"),
                    m("41/29"),
                    t(", ", "、"),
                    m("99/70"),
                    t(". Because ", "があります。"),
                    m("17^2 - 2*12^2 = 1"),
                    t(",", "なので、"),
                ]),
                StudyPart::Equation("17 - 12*sqrt(2) = 1/(17 + 12*sqrt(2))."),
                p(&[
                    t("At Studio ", "Studioの"),
                    m("t = 12"),
                    t(", the remaining phase is about ", "では、残る位相は約"),
                    m("-0.184960"),
                    t(
                        " radians and the normalized state error is ",
                        "ラジアン、正規化した状態の誤差は",
                    ),
                    m("0.184696"),
                    t(". At ", "です。"),
                    m("t = 29"),
                    t(" it is ", "では"),
                    m("0.076594"),
                    t("; at ", "、"),
                    m("t = 70"),
                    t(" it is ", "では"),
                    m("0.031733"),
                    t(
                        ". Each is closer, none is an exact return. The capsule currently stops at ",
                        "です。順に近づいていますが、どれも厳密な復帰ではありません。現在のカプセルは",
                    ),
                    m("12"),
                    t(
                        "; the later times require extending its saved domain.",
                        "までで終わるので、それより後の時刻を見るには、保存された定義域を広げる必要があります。",
                    ),
                ]),
                p(&[
                    t(
                        "These continued-fraction convergents obey ",
                        "この連分数の近似分数は",
                    ),
                    m("abs(r-p_k/q_k) < 1/(q_k*q_(k+1))"),
                    t(
                        ", so their full-state error is below ",
                        "を満たすので、状態全体の誤差は",
                    ),
                    m("2*pi/q_(k+1)"),
                    t(
                        ". Now design a tolerance first and choose a time window to meet it.",
                        "未満です。今度は許容誤差を先に決め、それを満たす時間区間を選んでみましょう。",
                    ),
                ]),
                r(
                    &CONTINUED_FRACTIONS,
                    "Convergent error bounds.",
                    "近似分数の誤差上界。",
                ),
            ],
        ),
        block(
            locale,
            "lissajous.sound",
            "What the sound carries",
            "音が伝えるもの",
            StudyDepth::Mathematics,
            vec![
                p(&[
                    t(
                        "The Lissajous room targets two audio frequencies, approximately ",
                        "Lissajousルームが音の目標にするのは、約",
                    ),
                    m("110*a"),
                    t(" and ", "Hzと"),
                    m("110*b"),
                    t(" Hz. Thus ", "Hzという二つの周波数です。"),
                    m("2:3"),
                    t(" selects ", "では"),
                    m("220"),
                    t(" and ", "Hzと"),
                    m("330"),
                    t(" Hz, and ", "Hz、"),
                    m("4:6"),
                    t(" selects ", "では"),
                    m("440"),
                    t(" and ", "Hzと"),
                    m("660"),
                    t(
                        " Hz. The ratio is unchanged while both notes rise one octave. The ",
                        "Hzになります。比は変わらず、両方の音が一オクターブ上がります。周波数比",
                    ),
                    m("3:2"),
                    t(
                        " frequency ratio is the just perfect fifth; it is about ",
                        "は純正完全五度で、約",
                    ),
                    m("1.955"),
                    t(
                        " cents wider than seven semitones of twelve-tone equal temperament. The calculation is ",
                        "セントだけ十二平均律の七半音より広い音程です。計算式は",
                    ),
                    m("1200*log2(3/2) - 700"),
                    t(".", "です。"),
                ]),
                r(
                    &ACOUSTICS,
                    "The logarithmic pitch convention: frequency ratios, octaves, semitones and cents.",
                    "対数による音高の表し方。周波数比、オクターブ、半音、セント。",
                ),
                p(&[
                    t(
                        "The live mathematical voice preserves its own oscillator phases and glides both absolute frequencies toward new targets. It does not copy the drawing's phases. A changing ellipse at fixed frequencies need not change this voice's pitch. The one-shot chord lasts ",
                        "数式に対応するリアルタイムの音は、それ自身の振動子の位相を保ちながら、二つの周波数をそれぞれ新しい目標値へ滑らかに近づけます。図形の位相をコピーするわけではありません。角振動数を固定したまま楕円が変化しても、この音の高さが変わるとは限りません。単発の和音は、音量が時間とともに変化する",
                    ),
                    m("1.5"),
                    t(
                        " seconds with an envelope; its complete audio buffer is not an indefinitely repeating ideal orbit.",
                        "秒の音です。その音声バッファ全体は、無限に繰り返される理想軌道ではありません。",
                    ),
                ]),
                p(&[t(
                    "Room score, radio, other voices, mute, and device or host support affect what is heard. A ratio is measurable; whether a listener enjoys it is not a theorem about small integers.",
                    "ルームの楽曲、ラジオ、ほかの音、ミュート、デバイスやホストの対応状況も、何が聴こえるかに影響します。比は測定できますが、その音を楽しめるかどうかは、小さな整数についての定理からは決まりません。",
                )]),
                p(&[t(
                    "Studio is a different instrument: its current melody maps sampled y values through a pitch map. Opening a Lissajous-shaped capsule does not automatically sonify its two coordinate frequencies as this room does.",
                    "Studioは別の楽器です。現在のメロディーは、サンプリングしたyの値を音高マップで音に対応させます。リサジュー図形を描くカプセルを開いても、このルームのように二つの座標の角振動数が自動的に音になるわけではありません。",
                )]),
            ],
        ),
        block(
            locale,
            "lissajous.limits",
            "What this instrument actually computes",
            "この楽器が実際に計算していること",
            StudyDepth::Mathematics,
            vec![
                p(&[
                    t(
                        "With no accepted position input, ",
                        "有効な位置入力がないときは、",
                    ),
                    m("a = 3"),
                    t(" and ", "、"),
                    m("b = 2 + 3*t"),
                    t(", for bounded gallery ", "です。ギャラリーの範囲は"),
                    m("0 <= t <= 1"),
                    t(
                        ". After a position tuning, both angular frequencies are integers ",
                        "です。位置を操作して調律すると、両方の角振動数の設定値は",
                    ),
                    m("1"),
                    t(" through ", "から"),
                    m("8"),
                    t(
                        "; horizontal position selects b and vertical position selects a. Gallery t then advances the y phase by ",
                        "までの整数になります。横位置がbを、縦位置がaを選びます。その後のギャラリーtは、yの位相を",
                    ),
                    m("2*pi*frac(t)"),
                    t(
                        " rather than advancing a particle along one fixed trajectory. Variation changes initial phases.",
                        "だけ進めます。一つの固定軌道に沿って粒子を進めるのではありません。バリエーションは初期位相を変えます。",
                    ),
                ]),
                p(&[
                    t("Each frame draws ", "各フレームで描く線分は"),
                    m("1,500"),
                    t(" segments from ", "本で、"),
                    m("1,501"),
                    t(" samples over ", "個の標本から作られます。範囲は"),
                    m("0 <= theta <= 2*pi"),
                    t(
                        ". For integer position tunings this window contains ",
                        "です。位置を操作して整数に調律した場合、この区間で運動全体が",
                    ),
                    m("gcd(a,b)"),
                    t(
                        " complete periods. A sweep value can require a longer period than the window. The renderer connects consecutive samples and does not add an artificial final-to-first segment. Earlier distinct tunings can remain overlaid, so not every visible stroke belongs to the latest oscillator pair.",
                        "周期分繰り返されます。自動変化の途中の値では、周期が表示区間より長くなることがあります。描画処理は隣り合う標本を結び、終点から始点への線分を人為的に追加しません。以前の異なる調律が重ねて残る場合もあるので、見えているすべての線が最新の振動子の組に属するとは限りません。",
                    ),
                ]),
                p(&[
                    t("The continuous parameters use ", "連続パラメータの計算には"),
                    m("binary64"),
                    t(" arithmetic; audio targets use ", "を、音の目標値には"),
                    m("binary32"),
                    t(
                        ". Every finite angular frequency stored in binary floating point is rational as a real number. Therefore the fixed stored pair, interpreted as exact coefficients of ideal sine functions, has a rational ratio. That fact does not guarantee bitwise repetition of floating-point evaluations or rendered frames. Actual arithmetic, trigonometric evaluation, pixel rounding, and changing controls are distinct from the ideal model. Neither a finite trace nor a readout rounded to two decimal places proves irrationality or eternal nonrepetition. The ",
                        "を使います。有限の二進浮動小数点数で表された角振動数は、実数としてはすべて有理数です。したがって、保存された固定の角振動数の組を、理想的な正弦関数の厳密な係数として解釈すると、その比は有理数になります。ただし、それによって浮動小数点での評価結果や描画フレームがビット単位で繰り返される保証は得られません。実際の演算、三角関数の評価、画素への丸め、操作による設定変更は、理想モデルとは区別する必要があります。有限の軌跡も、小数第二位までに丸めた表示も、無理数であることや永遠に繰り返さないことの証明にはなりません。",
                    ),
                    m("sqrt(2)"),
                    t(
                        " construction specifies an irrational ideal model in Studio; its evaluation still approximates that model.",
                        "を使った構成は、Studio上で無理数を含む理想モデルを指定しますが、その評価もやはりモデルの近似です。",
                    ),
                ]),
                r(
                    &BINARY64,
                    "The representation contract for the room's floating-point parameters.",
                    "ルームの浮動小数点パラメータの表現に関する仕様。",
                ),
                r(
                    &GOLDBERG,
                    "Representation and numerical approximation.",
                    "数の表現と数値近似。",
                ),
                p(&[
                    t(
                        "There is a useful sampling bound, separately from these numerical effects. For exact sine/cosine samples, ",
                        "こうした数値計算の影響とは別に、サンプリングには有用な誤差上界があります。正弦・余弦の標本が厳密なら、",
                    ),
                    m("abs(q'') <= omega^2"),
                    t(
                        ", and linear interpolation on a step h has coordinate error at most ",
                        "であり、刻み幅hでの線形補間による座標の誤差は高々",
                    ),
                    m("omega^2*h^2/8"),
                    t(". With a and b at most ", "です。係数a,bが最大"),
                    m("8"),
                    t(" and ", "、"),
                    m("h = 2*pi/1500"),
                    t(", this is below ", "なら、振幅一の座標での誤差は"),
                    m("0.000141"),
                    t(
                        " in unit-amplitude coordinates. This bound excludes floating-point evaluation, rounding to pixels or text cells, line rasterization, and display effects; it is not a total screenshot error budget.",
                        "未満になります。この上界は、浮動小数点での評価、画素や文字セルへの丸め、線のラスタ化、表示効果を含みません。スクリーンショット全体の誤差上限ではありません。",
                    ),
                ]),
                r(
                    &INTERPOLATION,
                    "The interpolation remainder used for the coordinate bound.",
                    "座標の誤差上界に用いる補間剰余項。",
                ),
            ],
        ),
        block(
            locale,
            "lissajous.references",
            "Follow the proofs",
            "証明をたどる",
            StudyDepth::Mathematics,
            vec![
                r(
                    &MIT,
                    "Lissajous motion, periodic and quasiperiodic contrasts, retracing, and the limitation of representing irrational coefficients numerically.",
                    "リサジュー運動、周期的な場合と準周期的な場合の対比、同じ軌跡をたどり直すこと、無理数の係数を数値で表す際の制約。",
                ),
                r(
                    &ERGODIC,
                    "Fourier proof of irrational-rotation equidistribution and the uniform approximation step used by the continuous torus calculation.",
                    "無理数回転の一様分布性のフーリエ解析による証明と、連続時間のトーラス上の計算で使う一様近似の手順。",
                ),
                r(
                    &DIRICHLET,
                    "Dirichlet approximation and its recurrence interpretation.",
                    "ディリクレ近似と、再帰としての解釈。",
                ),
                r(
                    &CONTINUED_FRACTIONS,
                    "Convergent error bounds.",
                    "近似分数の誤差上界。",
                ),
                r(&INTERPOLATION, "Interpolation remainder.", "補間剰余項。"),
                r(
                    &ACOUSTICS,
                    "Frequency ratios, octaves, equal-tempered semitones, and cents.",
                    "周波数比、オクターブ、平均律の半音、セント。",
                ),
                r(
                    &BINARY64,
                    "Binary floating-point representation.",
                    "二進浮動小数点数の表現。",
                ),
                r(
                    &GOLDBERG,
                    "Representation and numerical approximation.",
                    "数の表現と数値近似。",
                ),
            ],
        ),
    ]
}

static MIT: StudySource = StudySource {
    id: "mit-18-353-lissajous",
    title: "MIT 18.353, problem-set answers, section 3 (2024)",
    url: "https://math.mit.edu/classes/18.353J/PSetAnswers/AnswerPSet_2024_07.pdf",
};

static ERGODIC: StudySource = StudySource {
    id: "oxford-ergodic-theory",
    title: "Ben Green, Ergodic Theory, chapter 1 and appendix B (2015)",
    url: "https://people.maths.ox.ac.uk/greenbj/papers/ergodic-2015.pdf",
};

static DIRICHLET: StudySource = StudySource {
    id: "uncg-dirichlet-recurrence",
    title: "UNCG Ergodic Theory Summer School, lesson 8 (2020)",
    url: "https://bpb-us-w2.wpmucdn.com/sites.uml.edu/dist/2/372/files/2021/07/UNCG_Ergodic_Theory_Summer_School_2020.pdf",
};

static CONTINUED_FRACTIONS: StudySource = StudySource {
    id: "oxford-continued-fractions",
    title: "Ben Green, Continued Fractions",
    url: "https://people.maths.ox.ac.uk/greenbj/papers/continued-fraction.pdf",
};

static INTERPOLATION: StudySource = StudySource {
    id: "nist-dlmf-3-3-5",
    title: "NIST Digital Library of Mathematical Functions, equation 3.3.5",
    url: "https://dlmf.nist.gov/3.3.E5",
};

static ACOUSTICS: StudySource = StudySource {
    id: "unsw-notes-and-frequencies",
    title: "UNSW Music Acoustics: notes and frequencies",
    url: "https://newt.phys.unsw.edu.au/jw/notes.html",
};

static BINARY64: StudySource = StudySource {
    id: "rust-binary64",
    title: "Rust binary64 documentation",
    url: "https://doc.rust-lang.org/std/primitive.f64.html",
};

static GOLDBERG: StudySource = StudySource {
    id: "goldberg-floating-point",
    title: "David Goldberg, What Every Computer Scientist Should Know About Floating-Point Arithmetic",
    url: "https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html",
};
