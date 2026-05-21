/* 004459bf __setenvp */

/* Library Function - Single Match
    __setenvp
   
   Library: Visual Studio 2003 Release */

int __cdecl __setenvp(void)

{
  undefined4 *puVar1;
  size_t sVar2;
  uint *puVar3;
  uint *puVar4;
  int iVar5;
  
  if (DAT_0046cd4c == 0) {
    ___initmbctable();
  }
  iVar5 = 0;
  puVar4 = DAT_0046c558;
  if (DAT_0046c558 != (uint *)0x0) {
    for (; (char)*puVar4 != '\0'; puVar4 = (uint *)((int)puVar4 + sVar2 + 1)) {
      if ((char)*puVar4 != '=') {
        iVar5 = iVar5 + 1;
      }
      sVar2 = _strlen((char *)puVar4);
    }
    puVar1 = _malloc(iVar5 * 4 + 4);
    puVar4 = DAT_0046c558;
    DAT_0046c700 = puVar1;
    if (puVar1 != (undefined4 *)0x0) {
      do {
        if ((char)*puVar4 == '\0') {
          _free(DAT_0046c558);
          DAT_0046c558 = (uint *)0x0;
          *puVar1 = 0;
          DAT_0046cd40 = 1;
          return 0;
        }
        sVar2 = _strlen((char *)puVar4);
        if ((char)*puVar4 != '=') {
          puVar3 = _malloc(sVar2 + 1);
          *puVar1 = puVar3;
          if (puVar3 == (uint *)0x0) {
            _free(DAT_0046c700);
            DAT_0046c700 = (undefined4 *)0x0;
            return -1;
          }
          FUN_0043ebd0(puVar3,puVar4);
          puVar1 = puVar1 + 1;
        }
        puVar4 = (uint *)((int)puVar4 + sVar2 + 1);
      } while( true );
    }
  }
  return -1;
}
