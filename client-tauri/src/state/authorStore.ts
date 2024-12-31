import { makeObservable, action, observable } from 'mobx'

/**
 * Todos:
 * - Add online status with authors
 * - Add channels that know which authors they contain
 * - Add author cache that deletes authors when authors aren't present anymore
 */

export enum UserStatus {
  Offline,
  Online,
}

export interface User {
  name: string
  avatarURL: string
  status: UserStatus
}

export type UserRef = number

export class UserStore {
  // TODO: Enforce uniqueness through keys
  users: User[] = []

  constructor() {
    makeObservable(this, {
      users: observable,
      addUser: action,
      modifyUser: action,
    })

    this.addUser(dummyUser('haadi'))
    this.addUser(dummyUser('haadi2'))
  }

  addUser(user: User) {
    this.users.push(user)
  }

  getAuthor(userRef: UserRef): User {
    return this.users[userRef]
  }

  modifyUser(userRef: UserRef, userAttrs: Partial<User>) {
    this.users[userRef] = { ...this.users[userRef], ...userAttrs }
  }
}

const userStore = new UserStore()
export default userStore

function dummyUser(name: string, status?: UserStatus): User {
  return {
    name,
    status: status ?? UserStatus.Online,
    avatarURL:
      'https://cdn.discordapp.com/avatars/295595875168813058/9480345ded6da896d57957539bd4f881.webp?size=80',
  }
}
