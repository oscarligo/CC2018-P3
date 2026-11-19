//! Authored identity cards for the actual bodies in the scene, including their moons.
use crate::interaction::Interaction;

macro_rules! card {
    ($name:literal,$title:literal,$body:literal) => {
        Interaction {
            prompt: concat!("[F] Examinar ", $name),
            response: concat!($name, " · ", $title, "\n", $body),
        }
    };
}

pub(super) fn catalog(index: usize) -> Interaction {
    match index {
        0 => card!(
            "Sol principal",
            "El corazón cautivo",
            "El fuego que da color a todo el sistema. Dos aros de Dyson bailan a su alrededor: por dentro, espejos; por fuera, una coraza oscura contra el resplandor."
        ),
        1 => card!(
            "Protoplaneta",
            "El mundo inacabado",
            "Una silueta pequeña y accidentada, sembrada de rocas. Parece un fragmento de la creación que todavía no ha decidido qué forma tendrá."
        ),
        2 => card!(
            "Magmático",
            "Brasa errante",
            "Tonos de cobre y naranja arden bajo un velo cálido. Sus rocas recortan el horizonte como las últimas costras de un mundo que aún se está enfriando."
        ),
        3 => card!(
            "Invernadero",
            "El secreto amarillo",
            "Una envoltura dorada esconde la superficie entre nubes. Desde lejos parece apacible; de cerca, su luz espesa convierte cada vuelta en un misterio."
        ),
        4 => card!(
            "Oceánico",
            "El espejo azul",
            "Agua, tierra y nubes se mezclan bajo un borde azul. Su luna Marea acompaña este pequeño refugio de color entre las sombras del sistema."
        ),
        5 => card!(
            "Luna Marea",
            "La vigía del océano",
            "Pequeña, gris e irregular, vigila al Oceánico desde una órbita cercana. Su piel mineral contrasta con el azul de su compañero."
        ),
        6 => card!(
            "Desierto con anillo",
            "La corona de arena",
            "Un mundo rojizo luce un anillo inclinado como una joya antigua. Dos pequeñas lunas lo escoltan, cada una a su propio ritmo."
        ),
        7 => card!(
            "Luna Ascua",
            "La escolta veloz",
            "Una roca oscura que da vueltas rápidas al Desierto con anillo. Es la más cercana de sus dos compañeras, siempre intentando adelantar a la otra."
        ),
        8 => card!(
            "Luna Polvo",
            "La escolta paciente",
            "Más pequeña y distante que Ascua, sigue una vuelta pausada alrededor del mundo rojizo. Desde aquí, el anillo parece una línea dibujada en el vacío."
        ),
        9 => card!(
            "Planeta enano",
            "El náufrago mineral",
            "Un cuerpo gris y alargado rodeado de piedra. Sin el abrigo de una atmósfera, cada saliente queda expuesto a la luz y a la oscuridad."
        ),
        10 => card!(
            "Supertierra",
            "La esmeralda coronada",
            "Grande, verdosa y ligeramente achatada, lleva una corona de anillos. Una luna azulada completa la estampa de este jardín imaginario."
        ),
        11 => card!(
            "Luna Aguamarina",
            "La joya de la esmeralda",
            "Un satélite de tonos fríos envuelto en un borde azul. Su vuelta alrededor de la Supertierra ofrece una segunda perspectiva de los anillos."
        ),
        12 => card!(
            "Ureski",
            "La primavera que nunca termina",
            "Un rostro desértico mira siempre al sol; el opuesto duerme bajo una tundra azul. Entre ambos, un cinturón de primavera perpetua alberga copas escalonadas, helechos luminosos, saltarines de antenas doradas y mantas que revolotean. Ureski está sincronizado con su estrella: aquí el atardecer es un lugar, no una hora."
        ),
        13 => card!(
            "Planeta Árido",
            "El silencio pálido",
            "Una esfera clara con rocas dispersas y un velo tenue. Sus colores recuerdan al polvo que queda cuando una tormenta termina y todo vuelve a quedarse quieto."
        ),
        14 => card!(
            "Coloso Joviano",
            "El rey de las bandas",
            "Franjas y remolinos recorren este gigante achatado. Un anillo y tres lunas forman su corte: Azufre, Escarcha y Cantera."
        ),
        15 => card!(
            "Luna Azufre",
            "La chispa del coloso",
            "Una compañera amarillenta con rocas y un halo dorado. Su órbita rápida la hace destacar entre las tres lunas del Coloso Joviano."
        ),
        16 => card!(
            "Luna Escarcha",
            "La perla fría",
            "Redonda y azulada, aporta una nota de calma junto a las bandas cálidas del Coloso. No necesita anillos para parecer una joya."
        ),
        17 => card!(
            "Luna Cantera",
            "El guardián de piedra",
            "La mayor de las tres lunas jovianas tiene un contorno áspero y gris. Su recorrido exterior parece custodiar a las dos compañeras más pequeñas."
        ),
        18 => card!(
            "Gigante Anillado Imperial",
            "El trono orbital",
            "Su amplio sistema de anillos domina la silueta de este gigante crema. Dos lunas de tonos opuestos orbitan alrededor de su corona inclinada."
        ),
        19 => card!(
            "Luna Ámbar",
            "La linterna imperial",
            "Un satélite cálido con una envoltura anaranjada. Cerca del gigante anillado parece una pequeña lámpara encendida en un palacio vacío."
        ),
        20 => card!(
            "Luna Nácar",
            "La hermana del hielo",
            "Clara y azulada, contrasta con el brillo cálido de Ámbar. Un velo frío y las rocas de su entorno le dan una presencia delicada."
        ),
        21 => card!(
            "Enana Marrón",
            "La brasa con corona",
            "Un segundo fuego, más rojizo, viaja por las regiones exteriores. Sus anillos y su luna Carbón la convierten en una pequeña corte aparte."
        ),
        22 => card!(
            "Luna Carbón",
            "La sombra de la brasa",
            "Oscura, rojiza e irregular, acompaña a la Enana Marrón. Su silueta parece un pedazo de carbón que se resiste a apagarse."
        ),
        23 => card!(
            "Gigante Cian",
            "El equilibrista",
            "Un gigante frío de tonos cian lleva un anillo casi vertical. Su inclinación hace que parezca desafiar el equilibrio del resto del sistema."
        ),
        24 => card!(
            "Luna Astilla",
            "La compañera del equilibrista",
            "Una roca pálida y alargada rodea al Gigante Cian. Frente al enorme anillo inclinado, su tamaño recuerda lo monumental que es su vecino."
        ),
        25 => card!(
            "Gigante Cobalto",
            "El océano de tormentas",
            "Azules profundos y bandas inquietas cubren este gigante sin anillo. Su luna Contracorriente añade una rareza: se mueve en sentido retrógrado."
        ),
        26 => card!(
            "Luna Contracorriente",
            "La rebelde",
            "Gira y orbita en sentido retrógrado alrededor del Gigante Cobalto. Mientras las demás parecen seguir un acuerdo secreto, ella toma su propio camino."
        ),
        27 => card!(
            "Alfa Riddertal",
            "El archivo que devuelve la mirada",
            "Nubes rosadas velan una entidad de apariciones superpuestas y ojos imposibles. Seis estaciones consultan su memoria infinita: los pulsos cian llevan preguntas y los rosados devuelven ecos del pasado humano. El núcleo cambia al observarlo. Nadie sabe si recuerda nuestra historia o si la está soñando."
        ),
        28 => card!(
            "Luna Gemela",
            "La compañera del confín",
            "Una luna gris custodia el archivo rosado de Alfa Riddertal, por fuera de sus estaciones de enlace. Algunos exploradores dicen que es la única cosa cercana al planeta que nunca responde a una pregunta."
        ),
        29 => card!(
            "Gigante Lejano",
            "El centinela violeta",
            "Un gigante de tonos púrpura cierra el recorrido entre anillos y rocas. Desde su distancia, el sol y la estructura de Dyson parecen una sola joya remota."
        ),
        _ => unreachable!("body outside the authored catalog"),
    }
}

pub(super) fn named(name: &str) -> Option<Interaction> {
    match name {
        "Kashyyyk" => Some(card!(
            "Kashyyyk",
            "La catedral verde",
            "Árboles gigantes emergen entre costas, bosques y nubes. Busca la aldea: sus cabañas, plataformas y puentes se aferran a las copas como un pequeño hogar sobre el mundo."
        )),
        "Coruscant" => Some(card!(
            "Coruscant",
            "El planeta que no duerme",
            "Un océano de metal, calles y torres cubre este mundo-ciudad. En el lado nocturno, las luces ámbar y azules dibujan una constelación sobre la propia superficie."
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_authored_body_has_a_distinct_readable_identity() {
        let mut prompts = std::collections::HashSet::new();
        let mut descriptions = std::collections::HashSet::new();
        for item in (0..30)
            .map(catalog)
            .chain([named("Kashyyyk").unwrap(), named("Coruscant").unwrap()])
        {
            assert!(item.prompt.starts_with("[F] Examinar "));
            assert!(item.response.contains('\n'));
            assert!(prompts.insert(item.prompt));
            assert!(descriptions.insert(item.response));
        }
        assert_eq!(descriptions.len(), 32);
    }
}
