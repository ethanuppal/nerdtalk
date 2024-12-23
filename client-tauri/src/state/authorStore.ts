import { makeObservable, action, observable } from 'mobx'

export interface Author {
    name: string;
    avatarURL: string;
}

export type AuthorRef = number

export class AuthorStore {
    // TODO: Enforce uniqueness through keys
    authors: Author[] = []

    constructor() {
        makeObservable(this, {
            authors: observable,
            addAuthor: action,
        })

        this.addAuthor({
          name: 'Haadi',
          avatarURL:
            'https://cdn.discordapp.com/avatars/295595875168813058/9480345ded6da896d57957539bd4f881.webp?size=80',
        })        
        this.addAuthor({
          name: 'Haadi2',
          avatarURL:
            'https://cdn.discordapp.com/avatars/295595875168813058/9480345ded6da896d57957539bd4f881.webp?size=80',
        })
    }

    addAuthor(author: Author) {
        this.authors.push(author)
    }

    getAuthor(authorRef: AuthorRef): Author {
        return this.authors[authorRef];
    }
}

const authorStore = new AuthorStore()
export default authorStore