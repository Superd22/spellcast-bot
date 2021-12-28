use trees::{TreeWalk};
use trees::walk::{Visit};
use postgres::types::Type;
use postgres::{Client, NoTls};
use trees::{Node, Tree};

fn main() {
    env_logger::init();

    let mut client = Client::connect("host=localhost user=test password=test dbname=postgres", NoTls)
        .expect("Could not connect to db");

    let matrix = [
        [CharInMatrix{char: 'a', value: 1, word_multiplier: 1}, CharInMatrix{char: 'a', value: 1, word_multiplier: 1}, CharInMatrix{char: 'c', value: 1, word_multiplier: 1}, CharInMatrix{char: 'd', value: 1, word_multiplier: 1}, CharInMatrix{char: 'e', value: 1, word_multiplier: 1}],
        [CharInMatrix{char: 'f', value: 1, word_multiplier: 1}, CharInMatrix{char: 'r', value: 1, word_multiplier: 1}, CharInMatrix{char: 'h', value: 1, word_multiplier: 1}, CharInMatrix{char: 'i', value: 1, word_multiplier: 1}, CharInMatrix{char: 'j', value: 1, word_multiplier: 1}],
        [CharInMatrix{char: 'k', value: 1, word_multiplier: 1}, CharInMatrix{char: 'o', value: 1, word_multiplier: 1}, CharInMatrix{char: 'm', value: 1, word_multiplier: 1}, CharInMatrix{char: 'n', value: 1, word_multiplier: 1}, CharInMatrix{char: 'o', value: 1, word_multiplier: 1}],
        [CharInMatrix{char: 'p', value: 1, word_multiplier: 1}, CharInMatrix{char: 'n', value: 1, word_multiplier: 1}, CharInMatrix{char: 'r', value: 1, word_multiplier: 1}, CharInMatrix{char: 's', value: 1, word_multiplier: 1}, CharInMatrix{char: 't', value: 1, word_multiplier: 1}],
        [CharInMatrix{char: 'u', value: 1, word_multiplier: 1}, CharInMatrix{char: 'v', value: 1, word_multiplier: 1}, CharInMatrix{char: 'w', value: 1, word_multiplier: 1}, CharInMatrix{char: 'x', value: 1, word_multiplier: 1}, CharInMatrix{char: 'y', value: 1, word_multiplier: 1}],
    ];

    let mut words = vec![];

    for i in 0..(4*4) {
        words.extend(
            walk_tree(
                construct_exploration_tree(matrix, PositionInMatrix { x: i%5, y:  i/5 + (i%5 != 0) as isize }, &mut client),
                &matrix
            )
        )
    }

    words.sort_by(|a, b| b.value.cmp(&a.value));
    println!("{:#?}", words);
}

/**
 * Shared
 */
 fn find_char_in_matrix<'a>(
    matrix: &'a[[CharInMatrix; 5]; 5],
    position: &PositionInMatrix,
) -> &'a CharInMatrix {
    return &matrix[position.y as usize][position.x as usize]
}



/**
 * Tree exploration
 */
#[derive(Debug)]
struct Word {
    word: String,
    value: u16,
    path: Vec<PositionInMatrix>
}

fn walk_tree(
    tree: Tree<CharLeaf>,
    matrix: &[[CharInMatrix; 5]; 5]
)-> Vec<Word> {
    let mut walk = TreeWalk::from( tree );

    let mut words: Vec<Word> = vec![];
    let mut prefix: String = String::from("");
    let mut path: Vec<PositionInMatrix> = vec![];
    let mut current_value = 0;
    let mut multiplier = 1;

    fn add_if_word(
        words: &mut Vec<Word>,
        data: &CharLeaf,
        char_in_matrix: &CharInMatrix,
        path: &Vec<PositionInMatrix>,
        current_prefix: &String,
        current_value: u16,
        curent_multiplier: u16,
    ) {
        if data.is_word {
            let mut word_path = path.to_vec();
            word_path.push(data.pos);
            words.push(Word{
                word: format!("{prefix}{char}", prefix=current_prefix, char=data.char),
                path: word_path,
                value: (current_value + char_in_matrix.value as u16) * if char_in_matrix.word_multiplier > curent_multiplier { char_in_matrix.word_multiplier } else { curent_multiplier }
            })
        }
    }

    while let Some(visit) = walk.get() {
        let data: &CharLeaf = visit.node().data();
        let char_in_matrix = find_char_in_matrix(matrix, &data.pos);
        match visit {
            Visit::Begin(_) => {
                add_if_word(&mut words, &data, &char_in_matrix, &path, &prefix, current_value, multiplier);

                prefix.push(data.char);
                path.push(data.pos);
                if char_in_matrix.word_multiplier > 1 {
                    multiplier = char_in_matrix.word_multiplier;
                }
                current_value += char_in_matrix.value;
            },
            Visit::Leaf (_) => {
                add_if_word(&mut words, &data, &char_in_matrix, &path, &prefix, current_value, multiplier);
            },
            Visit::End  (_) => {
                prefix.pop();
                path.pop();
                if char_in_matrix.word_multiplier > 1 {
                    multiplier = 1
                }
                current_value -= char_in_matrix.value;
            },
        }
        walk.forward();
    }

    return words;   
}




/**
 * Tree construction
 */



#[derive(Debug, Copy, Clone, PartialEq)]
struct PositionInMatrix {
    x: isize,
    y: isize,
}

#[derive(Debug, Copy, Clone)]
struct CharLeaf {
    char: char,
    is_word: bool,
    pos: PositionInMatrix,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct CharInMatrix {
    char: char,
    value: u16,
    word_multiplier: u16,
}

#[derive(Debug)]
struct NextLetter {
    char: char,
    would_be_word: bool
}

impl std::fmt::Display for CharLeaf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CharLeaf ({})", self.char)
    }
}

fn construct_exploration_tree(
    matrix: [[CharInMatrix; 5]; 5],
    starting_position: PositionInMatrix,
    client: &mut Client,
) -> Tree<CharLeaf> {
    fn add_children_at_pos(
        tree: &mut Node<CharLeaf>,
        matrix:  [[CharInMatrix; 5]; 5],
        already_seen: Vec<PositionInMatrix>,
        client: &mut Client,
    ) {
        let prepare_search = |parents: &Vec<PositionInMatrix>| {
            let subpath = parents
                .iter()
                .map(|parent| format!("{}.", matrix[parent.y as usize][parent.x as usize].char))
                .collect::<String>();
            return format!(
                "SELECT 
                    subpath(path, {nlevel}, 1)::text AS nextLetter,
                    max(
                        (
                            path::text = concat(
                                '{subpath}',
                                subpath(path, {nlevel}, 1)::text
                            )
                        )::int
                    )::bool as isWord
                FROM test
                WHERE path ~ '{subpath}*'
                AND nlevel(path) > {nlevel}
                GROUP BY  nextLetter",
                subpath = subpath,
                nlevel = parents.len(),
            );
        };
        let mut possible_next_letters: Option<Vec<NextLetter>> = None;

        if already_seen.len() > 1 {
            let search_query = prepare_search(&already_seen);
            possible_next_letters = Some(client
                .query(&search_query, &[])
                .expect("Could not query pg")
                .iter()
                .map(|next_letter_row| {
                    let next_letter: String = next_letter_row.get(0);
                    return NextLetter{
                        char: next_letter.chars().take(1).last().unwrap(),
                        would_be_word: next_letter_row.get(1)
                    }
                }).collect()
            );
            log::debug!("Got results : {:#?}", possible_next_letters);
        }

        let existing_word_condition = |neighbor: PositionInMatrix| -> bool {
            if let Some(x) = &possible_next_letters {
                x.iter().any(|next_letter| { next_letter.char == matrix[neighbor.y as usize][neighbor.x as usize].char })
            } else { true }
        };
    
        let is_word = |letter: char| -> bool {
            if let Some(x) = &possible_next_letters {
            if let Some(found_letter) = x.iter().find(|next_letter| { next_letter.char == letter }) {
                    return found_letter.would_be_word;
                } else { false }
            } else { false }
        };

        for neighbor in find_neighbor(tree.data().pos) {

            if existing_word_condition(neighbor) &&
            !already_seen.iter().any(|parent| *parent == neighbor) 
            {
                let letter = matrix[neighbor.y as usize][neighbor.x as usize].char;
                tree.push_back(Tree::new(CharLeaf {
                    char: letter,
                    is_word: is_word(letter),
                    pos: neighbor,
                }));
            }
        }

        if !tree.has_no_child() {
            tree.iter_mut().for_each(|child| {
                let mut seen_for_child = already_seen.to_vec();
                let child_ref = child.get_mut();
                seen_for_child.push(child_ref.data().pos);
                add_children_at_pos(child_ref, matrix, seen_for_child, client)
            })
        }
    };

    let mut tree = Tree::new(CharLeaf {
        char: matrix[starting_position.y as usize][starting_position.x as usize].char,
        is_word: false,
        pos: starting_position,
    });

    add_children_at_pos(&mut tree.root_mut(), matrix, vec![starting_position], client);

    return tree;
}

fn find_neighbor(position: PositionInMatrix) -> Vec<PositionInMatrix> {
    let possible_neighbors = vec![
        PositionInMatrix {
            x: position.x - 1,
            y: position.y - 1,
        },
        PositionInMatrix {
            x: position.x,
            y: position.y - 1,
        },
        PositionInMatrix {
            x: position.x + 1,
            y: position.y - 1,
        },
        PositionInMatrix {
            x: position.x + 1,
            y: position.y,
        },
        PositionInMatrix {
            x: position.x + 1,
            y: position.y + 1,
        },
        PositionInMatrix {
            x: position.x,
            y: position.y + 1,
        },
        PositionInMatrix {
            x: position.x - 1,
            y: position.y + 1,
        },
        PositionInMatrix {
            x: position.x - 1,
            y: position.y,
        },
    ];

    return possible_neighbors
        .into_iter()
        .filter(|pos| pos.x >= 0 && pos.y >= 0 && pos.x < 5 && pos.y < 5)
        .collect();
}
