import { makeObservable, observable, action } from 'mobx'

export interface Author {
    name: string;
    avatarURL: string;
}

export class AuthorStore {
    authors: Set<Author> = new Set()

    constructor() {
        makeObservable(this, {
            authors: observable,
            addAuthor: action
        })
    }

    addAuthor(author: Author) {
        this.authors.add(author)
    }
}

const authorStore = new AuthorStore()
export default authorStore