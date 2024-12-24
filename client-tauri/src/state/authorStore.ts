import { makeObservable, action, observable } from 'mobx'

/**
 * Todos:
 * - Add online status with authors
 * - Add channels that know which authors they contain
 * - Add author cache that deletes authors when authors aren't present anymore
 */

export enum AuthorStatus {
  Offline,
  Online
}

export interface Author {
  name: string
  avatarURL: string
  status: AuthorStatus
}

interface AuthorOptions {
  name?: string
  avatarURL?: string
  status?: AuthorStatus
}

export type AuthorRef = number

export class AuthorStore {
  // TODO: Enforce uniqueness through keys
  authors: Author[] = []

  constructor() {
    makeObservable(this, {
      authors: observable,
      addAuthor: action,
      changeAuthor: action,
    })

    this.addAuthor(dummyAuthor('haadi'))
    this.addAuthor(dummyAuthor("haadi2"))
  }

  addAuthor(author: Author) {
    this.authors.push(author)
  }

  getAuthor(authorRef: AuthorRef): Author {
    return this.authors[authorRef]
  }

  changeAuthor(authorRef: AuthorRef, authorAttrs: AuthorOptions) {
    this.authors[authorRef] = { ...this.authors[authorRef], ...authorAttrs }
  }
}

const authorStore = new AuthorStore()
export default authorStore

function dummyAuthor(name: string, status?: AuthorStatus): Author {
  return {
    name,
    status: status ?? AuthorStatus.Online,
    avatarURL:
      'https://cdn.discordapp.com/avatars/295595875168813058/9480345ded6da896d57957539bd4f881.webp?size=80',
  }
}